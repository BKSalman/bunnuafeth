<p align='right' dir="rtl"><sub>
  عربي؟ <a href='AR-README.md' title='Arabic README'>كمل قراءة بالعربي (لهجة سعودية)</a>.
</sub></p>

# Bunnuafeth | بو النوافذ

Bunnuafeth is a personal Window manager written in Rust

# General philosophy
This window manager is specifically made for me,
so features are mostly gonna be focused on my own workflow, but if you have good suggestions,
or would just like to contribute, you can open an issue or a PR.

however it could be a not too bad base for a patched window manager like dwm

# Build

to build the window manager you can just build it with cargo

```bash
# debug build
cargo build

# release build (when you want to actually use it)
cargo build --release
```

# Run

the compiled binary name is `bunnu`

so you can add the following entry to your display manager/login manager

```
[Desktop Entry]
Name=Bunnuafeth
Comment=Bunnuafeth
Exec=path/to/bunnuafeth/bunnu
TryExec=path/to/bunnuafeth/bunnu
Type=Application
```

then open your display manager and run Bunnuafeth
