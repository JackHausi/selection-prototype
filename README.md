# selection-prototype
Using Amthyst testing selecting elements with a mouse click and enter move command with mouse click. 

This is just a prototype for trying out selecting and moving 2D elements by using [UiTransform](https://docs-src.amethyst.rs/master/amethyst_ui/struct.UiTransform.html) for each element and flagging them as selectable and interactable.

Select circle elements by using a left mouse click and order a selected element by using a right mouse click. Works okish but I will definitely look into other approaches.

Compile using the command:
```
cargo run --features "vulkan"
```
