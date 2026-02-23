{
  mkShell,
  wallrus,
  cargo,
  rustc,
  rust-analyzer,
  clippy,
  rustfmt,
}:

mkShell {
  inputsFrom = [ wallrus ];
  packages = [
    cargo
    rustc
    rust-analyzer
    clippy
    rustfmt
  ];
}
