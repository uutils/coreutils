# expr

```
expr [EXPRESSION]
expr [OPTIONS]
```

Print the value of `EXPRESSION` to standard output

## After help

Print the value of `EXPRESSION` to standard output.  A blank line below
separates increasing precedence groups.

`EXPRESSION` may be:

- `ARG1 | ARG2`: `ARG1` if it is neither null nor 0, otherwise `ARG2`
- `ARG1 & ARG2`: `ARG1` if neither argument is null or 0, otherwise 0
- `ARG1 < ARG2`: `ARG1` is less than `ARG2`
- `ARG1 <= ARG2`: `ARG1` is less than or equal to `ARG2`
- `ARG1 = ARG2`: `ARG1` is equal to `ARG2`
- `ARG1 != ARG2`: `ARG1` is unequal to `ARG2`
- `ARG1 >= ARG2`: `ARG1` is greater than or equal to `ARG2`
- `ARG1 > ARG2`: `ARG1` is greater than `ARG2`
- `ARG1 + ARG2`: arithmetic sum of `ARG1` and `ARG2`
- `ARG1 - ARG2`: arithmetic difference of `ARG1` and `ARG2`
- `ARG1 * ARG2`: arithmetic product of `ARG1` and `ARG2`
- `ARG1 / ARG2`: arithmetic quotient of `ARG1` divided by `ARG2`
- `ARG1 % ARG2`: arithmetic remainder of `ARG1` divided by `ARG2`
- `STRING : REGEXP`: anchored pattern match of `REGEXP` in `STRING`
- `match STRING REGEXP`: same as `STRING : REGEXP`
- `substr STRING POS LENGTH`: substring of `STRING`, `POS` counted from 1
- `index STRING CHARS`: index in `STRING` where any `CHARS` is found, or 0
- `length STRING`: length of `STRING`
- `+ TOKEN`: interpret `TOKEN` as a string, even if it is a keyword like `match`
  or an operator like `/`
- `( EXPRESSION )`: value of `EXPRESSION`

Beware that many operators need to be escaped or quoted for shells.
Comparisons are arithmetic if both ARGs are numbers, else lexicographical.
Pattern matches return the string matched between \( and \) or null; if
\( and \) are not used, they return the number of characters matched or 0.

Exit status is `0` if `EXPRESSION` is neither null nor `0`, `1` if `EXPRESSION`
is null or `0`, `2` if `EXPRESSION` is syntactically invalid, and `3` if an
error occurred.

Environment variables:

- `EXPR_DEBUG_TOKENS=1`: dump expression's tokens
- `EXPR_DEBUG_RPN=1`: dump expression represented in reverse polish notation
- `EXPR_DEBUG_SYA_STEP=1`: dump each parser step
- `EXPR_DEBUG_AST=1`: dump expression represented abstract syntax tree
