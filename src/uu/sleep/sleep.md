# sleep

```
sleep NUMBER[SUFFIX]...
sleep OPTION
```

Pause for NUMBER seconds.

## After Help

Pause for NUMBER seconds.  SUFFIX may be 's' for seconds (the default),
'm' for minutes, 'h' for hours or 'd' for days.  Unlike most implementations
that require NUMBER be an integer, here NUMBER may be an arbitrary floating
point number.  Given two or more arguments, pause for the amount of time
specified by the sum of their values.
