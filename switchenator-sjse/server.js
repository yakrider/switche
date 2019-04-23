const express = require('express');
const path = require('path');

// create the web server app
let app = express();

// get the index of the first command line argument
const argv = process.argv.slice(
    process.argv.findIndex((arg) => !path.relative(arg, __filename)) + 1
);

// optionally allow the port to be set
const port = parseInt(argv[0], 10) || 8080;

// serve static files
app.use(express.static('web'));

// get the landing page
app.get('/', function (req, res) {
    res.sendFile('index.html', {
        root: './web'
    });
});

// start the server
app.listen(port, function () {
    console.log(`Server started on port ${port}!`);
});
