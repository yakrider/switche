const webpack = require('webpack');
const path = require('path');
const fs = require('fs');

var nodeModules = {};
fs.readdirSync('node_modules')
  .filter(function(x) {
    return ['.bin'].indexOf(x) === -1;
  })
  .forEach(function(mod) {
    nodeModules[mod] = 'commonjs ' + mod;
  });

// where the final bundle be written
const outputPath = path.resolve('../../../../web/js');

// load the auto-generated webpack config file
module.exports = require('./scalajs.webpack.config');
module.exports.target = 'node';
module.exports.externals = nodeModules;

// modify the output directory where the bundle.js file is written
module.exports.output = {
    filename: 'switchenator-sjse-bundle.js',
    path: outputPath,
};

