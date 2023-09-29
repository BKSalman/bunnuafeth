# Extended Window Manager Hints (EWMH) checklist

things on this list are planned, otherwise it's not planned

#### atoms:
  root atoms:
  - [x] _NET_CLIENT_LIST
    - [x] update when mapping a new window
    - [x] update when unmapping/destroying a window
  - [ ] _NET_NUMBER_OF_DESKTOPS ([need to add tags/workspaces first](https://github.com/BKSalman/bunnuafeth/issues/4))
  - [x] _NET_DESKTOP_GEOMETRY
  - [x] _NET_DESKTOP_VIEWPORT
  - [ ] _NET_CURRENT_DESKTOP ([need to add tags/workspaces first](https://github.com/BKSalman/bunnuafeth/issues/4))
  - [ ] _NET_DESKTOP_NAMES ([need to add tags/workspaces first](https://github.com/BKSalman/bunnuafeth/issues/4))
  - [ ] _NET_ACTIVE_WINDOW
  - [ ] _NET_WORKAREA
    - [ ] update when mapping a dock
    - [ ] update when unmapping a dock
  - [x] _NET_SUPPORTING_WM_CHECK

  client atoms:
  - [x] _NET_WM_WINDOW_TYPE
  - [ ] _NET_WM_STATE
    - [ ] update when setting a fullscreen window
    - [ ] _NET_WM_STRUT_PARTIAL
    - [x] _NET_FRAME_EXTENTS

#### client messages:
- [ ] _NET_WM_STATE
  - [ ] update when setting a fullscreen window
