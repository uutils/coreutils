stty-usage = "stty [-F DEVICE | --file=DEVICE] [SETTING]...
  or:  stty [-F DEVICE | --file=DEVICE] [-a|--all]
  or:  stty [-F DEVICE | --file=DEVICE] [-g|--save]"

stty-about = "Print or change terminal characteristics."

stty-option-all = "print all current settings in human-readable form"
stty-option-save = "print all current settings in a stty-readable form"
stty-option-file = "open and use the specified DEVICE instead of stdin"
stty-option-settings = "settings to change"

stty-error-options-mutually-exclusive = "the options for verbose and stty-readable output styles are mutually exclusive"
stty-error-output-style-no-modes = "when specifying an output style, modes may not be set"
stty-error-missing-argument = "missing argument to '{$arg}'"
stty-error-invalid-speed = "invalid {$arg} '{$speed}'"
stty-error-invalid-argument = "invalid argument '{$arg}'"
stty-error-invalid-integer-argument = "invalid integer argument: {$value}"
stty-error-invalid-integer-argument-value-too-large = "invalid integer argument: {$value}: Value too large for defined data type"

# Output format strings
stty-output-speed = speed {$speed} baud;
stty-output-rows-columns = rows {$rows}; columns {$columns};
stty-output-line = line = {$line};
stty-output-undef = <undef>
stty-output-min-time = min = {$min}; time = {$time};
