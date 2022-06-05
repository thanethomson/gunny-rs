# Gunnyscript

Gunnyscript is a simple, strictly structured markup language that supports
everything that JSON does, but also:

- Built-in date and date/time support
- Docstrings for capturing descriptions of values that can be made accessible
  to users after processing
- Comments for people reading the markup itself

## Example

```
/// This is a docstring describing the following object
{
    // This comment will be ignored by the parser.

    /// The property "nothing" is a simple null value
    nothing null

    // Boolean values
    bool1 true  /// A simple boolean value, set to true
    bool2 false /// A simple boolean value, set to false

    // Numbers
    num       1     /// A simple positive number.
    pi        3.141 /// Floating point number.
    neg-num   -1    /// Negative number.

    // Strings
    hello      "world"
    escapes    "This string will contain\na newline"
    literal    #"This string will not contain\na newline"#
    litinlit   ##"A literal string #"within a literal string"#"##

    /// A simple multi-line string
    multiline1 "This string
flows across
multiple lines, preserving
the newlines."
    /// A dedented multi-line string
    multiline2 d"
        This string will be trimmed at the ends and will be dedented
        according to the indentation of the first non-empty line of text.
        Escaped characters\nwill be interpreted.

            This text will only be partially dedented.

        This text will be fully dedented.
    "
    /// A dedented multi-line literal string
    multiline3 d#"
        A multi-line literal that will preserve\nescaped characters
        and will honor explicit newlines, while also dedenting the text.
    "#

    // Dates
    date      2000-01-01               /// A date value in RFC3999 format.
    datetime  2000-01-01T00:00:00-0500 /// A date/time value in RFC3999 format.

    // Arrays
    arr1 [1, 2, 3, 4, 5]
    arr2 ["hello", "world", 1, 2, 3] /// Arrays can contain heterogeneous value types.

    // Objects
    obj1 {
        name        "Object 1"
        description "A simple object with a few properties"

        /// A nested object within obj1.
        nested {
            name "Nested object"
            arr  [1, 2, 3]
        }
    }

    // Arrays of objects
    objarr [
        {
            id    "one"
            value 1
        },
        {
            id    "two"
            value 2
        },
        {
            id    "three"
            value 3
        }
    ]
}
```

## Grammar

For a specification of the grammar, see [grammar.abnf](./grammar.abnf).

