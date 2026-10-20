#[doc(hidden)]
#[macro_export]
macro_rules! sec {
    ( 
        $ ( 
            $original:expr $(,)?
            $ (, $replaced:expr ) ? 
        ) ; * $(;)? 
    ) => {{ 
        vec![ 
            $( 
                Section { 
                    original: $original, 
                    replaced: None $(.or(Some($replaced)) )?.unwrap_or(Section::keep),
                },
                
            )* 
        ] 
    }};
}

/// Declarative macro for constructing one or an array of [`ChangeSpec`](crate::ChangeSpec) instances.
///
/// Because [`ChangeSpec::new`](crate::ChangeSpec::new) validates that `from <= to` and returns a
/// `Result`, invoking `ch!` uses the `?` operator in the surrounding function.
///
/// # Supported Forms
///
/// - **Single change (with or without braces):**
///   - `ch!({ from: 2, to: 5, insert: "abc" })`
///   - `ch!({ from: 2, insert: "abc" })` (insertion at `2`)
///   - `ch!({ from: 2, to: 5 })` (deletion of `2..5`)
/// - **Multiple changes:**
///   - `ch!([ { from: 0, insert: "a" }, { from: 3, to: 5 } ])`
///
/// # Examples
///
/// ```rust
/// use texera_core::{ChangeSet, Text, ch};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let single = ch!({ from: 0, to: 5, insert: "Hi" });
/// let multiple = ch!([
///     { from: 0, to: 5, insert: "Hi" },
///     { from: 5, insert: "!" },
/// ]);
///
/// let cs = ChangeSet::from_slice(&multiple, 5)?;
/// assert_eq!(cs.apply(Text::from("hello"))?.to_string(), "Hi!");
/// assert_eq!(single.from(), 0);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! ch {
    // 1. Canonical rule: contains the actual expansion logic only once
    (@item
        from: $from:expr$(, to: $to:expr)?$(, insert: $insert:expr)?$(,)?
    ) => {
        $crate::ChangeSpec::new($from,
            None $( .or(Some($to)) )?,
            None $( .or(Some($insert.into())) )?,
        )?
    };

    (@item
        insert: $insert:expr,
        from: $from:expr$(, to: $to:expr)?$(,)?
    ) => {
        $crate::ch!(@item from: $from$(, to: $to)?, insert:$insert)
    };

    (@item
        from: $from:expr,
        insert: $insert:expr,
        to: $to:expr$(,)?
    ) => {
        $crate::ch!(@item from:$from, to: $to, insert:$insert)
    };

    (@item
        insert: $insert:expr,
        to: $to:expr,
        from: $from:expr$(,)?
    ) => {
        $crate::ch!(@item from:$from, to: $to, insert:$insert)
    };

    // Guard to prevent infinite recursion if an invalid permutation is passed
    (@item $($invalid:tt)*) => {
        compile_error!("Invalid ChangeSpec syntax in `ch!`. Expected `from` (required), and optional `to` / `insert`.")
    };

    // Multiple changes: ch!([ { ... }, { ... } ])
    ([
        $( {$($body:tt)* } ),*$(,)?
    ] $(,)?) => {
        [ $($crate::ch!(@item $($body)*) ),* ]
    };

    // Single change with braces: ch!({ from: 0, insert: "a" })
    ({ $($body:tt)+ } $(,)?) => {$crate::ch!(@item $($body)+)
    };

    // Single change without braces: ch!(from: 0, insert: "a")
    ($($body:tt)+) => {$crate::ch!(@item $($body)+)
    };
}
