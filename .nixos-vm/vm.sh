#!/usr/bin/env bash

SCRIPTPATH=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

QEMU_OPTS="
  -vga qxl
  -spice unix=on,addr=/tmp/vm_spice.socket,disable-ticketing=on
  -device virtio-serial-pci
  -chardev spicevmc,id=spicechannel0,name=vdagent
  -device virtserialport,chardev=spicechannel0,name=com.redhat.spice.0
"

left-vm () {
  
  if [ "$1" = "clean" ]; then 

    rm -rf "${SCRIPTPATH}/result"
    rm "${SCRIPTPATH}/leftwm.qcow2"
    
  else

    echo building a leftwm virtual machine...
    cd "$SCRIPTPATH" # nixos-rebuild build-vm dumps result into current directory

    nixos-rebuild build-vm --flake ../#leftwm -L

    if [ $? -ne 0 ]; then
      exit $?
    fi

    export QEMU_OPTS="
      -spice unix=on,addr=/tmp/vm_spice.socket,disable-ticketing=on
      -vga none
      -device qxl-vga,vgamem_mb=64,ram_size_mb=256,vram_size_mb=128,max_outputs=2
      -display none
      -chardev spicevmc,id=charchannel0,name=vdagent
      -device virtio-serial-pci,id=virtio-serial0
      -device virtserialport,bus=virtio-serial0.0,nr=1,chardev=charchannel0,id=channel0,name=com.redhat.spice.0
    "
    ./result/bin/run-leftwm-vm & PID_QEMU="$!"
    sleep 1
    remote-viewer spice+unix:///tmp/vm_spice.socket
    kill $PID_QEMU
    cd -

  fi

}

