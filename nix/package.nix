{
  lib,
  rustPlatform,
  pkg-config,
  wrapGAppsHook4,
  gtk4,
  libadwaita,
  glib,
  cairo,
  pango,
  gdk-pixbuf,
  graphene,
  libglvnd,
  gsettings-desktop-schemas,
}:
rustPlatform.buildRustPackage {
  pname = "wallrus";
  version = "1.0.0";

  src = lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions [
      ./../Cargo.toml
      ./../Cargo.lock
      ./../src
      ./../data
    ];
  };

  cargoLock.lockFile = ./../Cargo.lock;

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    gtk4
    libadwaita
    glib
    cairo
    pango
    gdk-pixbuf
    graphene
    libglvnd
    gsettings-desktop-schemas
  ];

  postInstall = ''
    # Desktop file
    install -Dm644 data/io.github.megakode.Wallrus.desktop \
      $out/share/applications/io.github.megakode.Wallrus.desktop

    # Icon
    install -Dm644 data/icons/io.github.megakode.Wallrus.svg \
      $out/share/icons/hicolor/scalable/apps/io.github.megakode.Wallrus.svg

    # AppStream metainfo
    install -Dm644 data/io.github.megakode.Wallrus.metainfo.xml \
      $out/share/metainfo/io.github.megakode.Wallrus.metainfo.xml

    # Bundled palettes
    mkdir -p $out/share/wallrus/palettes
    cp -r data/palettes/* $out/share/wallrus/palettes/
  '';

  preFixup = ''
    gappsWrapperArgs+=(
      # Ensure libEGL.so.1 / libGLX.so.0 are available for dlopen at runtime
      --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [libglvnd]}"
    )
  '';

  meta = {
    description = "A GNOME application for generating abstract wallpapers using shaders";
    license = lib.licenses.gpl3Plus;
    platforms = lib.platforms.linux;
    mainProgram = "wallrus";
  };
}
