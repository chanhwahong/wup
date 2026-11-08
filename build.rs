fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/wup.ico");
    #[cfg(windows)]
    {
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let numeric = version.split('.').collect::<Vec<_>>().join(",");
        let icon_path = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("assets/wup.ico")
            .to_string_lossy()
            .replace('\\', "/");
        let resource = format!(
            r#"#include <winver.h>
1 ICON "{icon_path}"
1 VERSIONINFO
FILEVERSION {numeric},0
PRODUCTVERSION {numeric},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "WUp\0"
      VALUE "FileDescription", "WUp - WinGet Package Manager\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "wup\0"
      VALUE "OriginalFilename", "wup.exe\0"
      VALUE "ProductName", "WUp\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1200
  END
END
"#
        );
        let path =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("version.rc");
        std::fs::write(&path, resource).unwrap();
        embed_resource::compile(&path, embed_resource::NONE)
            .manifest_required()
            .expect("Windows version resource compilation failed");
    }
}
