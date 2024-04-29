

fn main() {

    // we'll set up a manifest to request elevated access before we build

    /*
    let mut win_attrs = tauri_build::WindowsAttributes::new();

    win_attrs = win_attrs.app_manifest (r#"
        <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
          <dependency>
            <dependentAssembly>
              <assemblyIdentity
                type="win32"
                name="Microsoft.Windows.Common-Controls"
                version="6.0.0.0"
                processorArchitecture="*"
                publicKeyToken="6595b64144ccf1df"
                language="*"
              />
            </dependentAssembly>
          </dependency>
          <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
            <security>
                <requestedPrivileges>
                    <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
                </requestedPrivileges>
            </security>
          </trustInfo>
        </assembly>
    "#);
    // */

    tauri_build::try_build(
        tauri_build::Attributes::new()  //.windows_attributes(win_attrs)
    ).expect("failed to run build script");

}
