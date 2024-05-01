

The project is in mostly two distinct portions:

#### Switche frontend:

The switche-front folder has the scala-js frontend for the project, and uses mill build tool as configured in build.sc.
There's a '*mill.bat*' which is actually '*millw.bat*'. 
To build, simply '*mill switche.webapp*' should be sufficient.
Doing a '*npm install*' during setup should pull the required js dependencies, including *tauri-api*.

It will create a compiled js file, and copy it along with resources to '*out/webapp*'.
Using the *-w* option with mill will watch for changes and recompile/copy dynamically.

If intend to use IntelliJ variant IDEs for dev, mill will have to generate the project files and dependency listings e.g. via '*mill mill.idea.GenIdea/idea*'.

Note that this mill-generated project will not work well if loaded as a module on some outer project due to issues with paths/dependencies defined there.  
Otoh, it works ok to load swtiche-back (as well as the outer project root itself)  as modules in the mill-generated project in switche-front.  
Alternately, can create separate intellij project for switche-back (or even outer project root) too and use them separately.
The nesting structure is not an issue as IntelliJ allows nested project roots. 

 
#### Switche backend:

The switche-back folder has the rust and tauri backend. 
With Tauri installed, it can be built via '*cargo tauri build*' for release version, which will generate the exe file as well as the installer.

For dev use, tauri is configured to use *vite* as a live-reload server, with its root pointing to the '*out/webapp*' folder in switche-front.  
Doing a '*npm install*' during setup should get *vite* setup. You'll need some recent nodejs installed. 

It can be ran via '*cargo tauri dev*', and will start a node server that serves the frontend.  
Changes to the rust backend code should trigger a rebuild, and changes to frontend contents in '*out/webapp*' should trigger a reload.


