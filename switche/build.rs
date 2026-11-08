fn main () {
    #[cfg(windows)] {
        let out_dir      = std::env::var ("OUT_DIR") .unwrap();
        let manifest_dir = std::env::var ("CARGO_MANIFEST_DIR") .unwrap();
        let version      = std::env::var ("CARGO_PKG_VERSION") .unwrap();

        let mut parts = version.split ('.') .map (|p| p.parse::<u16>() .unwrap_or (0));
        let (a, b, c) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0));

        let icon = std::path::Path::new (&manifest_dir) .join ("switche.ico");
        let rc_path = std::path::Path::new (&out_dir) .join ("switche.rc");

        let rc = format! (r#"
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {a},{b},{c},0
PRODUCTVERSION {a},{b},{c},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "Switche - Searchable Task Switcher\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "Switche\0"
      VALUE "OriginalFilename", "Switche.exe\0"
      VALUE "ProductName", "Switche\0"
      VALUE "ProductVersion", "{version}\0"
      VALUE "LegalCopyright", "Copyright (C) 2024 yakrider\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 0x4B0
  END
END
"#, icon = icon.display(), version = version, a = a, b = b, c = c);

        std::fs::write (&rc_path, rc) .expect ("failed to write resource file");
        println! ("cargo:rerun-if-changed=switche.ico");

        let _ = embed_resource::compile (&rc_path, embed_resource::NONE);
    }
}
