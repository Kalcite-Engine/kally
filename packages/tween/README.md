# tween

Small, allocation-free interpolation helpers for game and UI code.

## Usage

```klc
use tween;

i16 x = tween_step(0, 100, 4);
```

The package currently supports the stable Kally 0.14 language surface and uses
only integer arithmetic, so it is portable to desktop and NumWorks targets.
