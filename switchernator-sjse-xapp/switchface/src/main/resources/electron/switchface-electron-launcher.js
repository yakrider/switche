'use strict'; 
const {app, BrowserWindow} = require('electron');
require('electron-reload')(__dirname);
require("source-map-support").install(); 
require('./switchface-deps'); 
require('./switchface'); 
SwitchFace.SwitchFaceApp(__dirname, require).main();
