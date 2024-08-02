# Usage

Rendering mathematical equations with `pulldown-latex` obviously begins with using
the crate in order to generate the `mathml` markup. However, one also needs to include
the stylesheet and fonts necessary to render the equations correctly and aesthetically.

## Including the Required Files

Files can be included in one of two ways, either by using a CDN or by downloading them
from the release page and using them locally.

### Using a CDN

In the index.html file of your project, include the following lines:

```html
<head>
    <!-- Rest of your HTML head -->
    <!-- ...................... -->
    
    <!-- Include the Stylesheet -->
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/carloskiki/pulldown-latex@{version}/styles.min.css">
    <!-- Include the Fonts -->
    <link rel="preload" href="https://cdn.jsdelivr.net/gh/carloskiki/pulldown-latex@{version}/font/" as="font" crossorigin="anonymous">
    
    <!-- Rest of your HTML head -->
    <!-- ...................... -->
</head>
```

Make sure to replace `{version}` with the version of the crate you are using or with `latest` if you want to use the
latest version available.

### Using Local Files

Download the files from the [GitHub releases page](https://github.com/carloskiki/pulldown-latex/releases)
and include them in your project.

The `styles.css` file and the `font` directory should be placed together in the same directory.
You can change this structure by modifying the paths in the `styles.css` file.
