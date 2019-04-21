const webpack = require('webpack');
const path = require('path');

// where the final bundle be written
const outputPath = path.resolve('../../../../web/js');

// load the auto-generated webpack config file
module.exports = require('./scalajs.webpack.config');

// modify the output directory where the bundle.js file is written
module.exports.output = {
    filename: 'Sjseleclocaljstest-bundle.js',
    path: outputPath,
};
