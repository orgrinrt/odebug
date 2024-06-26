odebug
============
<div style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/stargazers)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/odebug)](https://crates.io/crates/odebug)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/issues)
[![Current Version](https://img.shields.io/badge/version-0.0.1-red.svg)](https://github.com/orgrinrt/odebug)

> Debug helper especially for proc-macros, that allows logging and outputting to a text file any various steps
> or info along the compilation process.

</div>

## Usage

TODO

## Example

TODO

## The Problem

Proc-macro crates are not the best at logging and printing useful stuff during compilation, and
often the final compilation error you get can be opaque or unhelpful.

There are some hurdles with this, and even IDEs don't really have a great support for debugging
this process. Stepping can lead to long, hard-to-follow tours inside the various popular proc-macro
crates like `syn` or `quote` or `proc-macro2`, and the process just isn't as fun as you'd hope.

In comes `odebug`, which allows you to select what you want to log/print, at which points, and
can take token streams as arguments out-of-the-box to give you real-time insight into what's going
on inside the proc-macro process.

Just use it like any regular old logger; call the desired variant of `debug_print` and after an error,
go see what the `odebug.txt` file has to say.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying
me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> You can check out the full license [here](https://github.com/orgrinrt/odebug/blob/master/LICENSE)

This project is licensed under the terms of the **MIT** license.
