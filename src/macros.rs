/// Macros for constructing `serde_yaml_bw::Value` literals.
///
/// These work similarly to `serde_json::json!` but produce YAML values.
///
/// # Examples
///
/// ```
/// use serde_yaml_bw::yaml;
///
/// let value = yaml!({
///     "name": "Ferris",
///     "age": 10,
///     "features": ["friendly", "productive"],
/// });
/// assert_eq!(serde_yaml_bw::to_string(&value).unwrap(), "name: Ferris\nage: 10\nfeatures:\n- friendly\n- productive\n");
/// ```
#[macro_export]
macro_rules! yaml {
    ($($yaml:tt)+) => {
        $crate::yaml_internal!($($yaml)+)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! yaml_internal {
    // Array parser
    (@array [$($elems:expr,)*]) => {
        $crate::__private::vec![$($elems,)*]
    };
    (@array [$($elems:expr),*]) => {
        $crate::__private::vec![$($elems),*]
    };
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!(null)] $($rest)*)
    };
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!(true)] $($rest)*)
    };
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!(false)] $($rest)*)
    };
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!([$($array)*])] $($rest)*)
    };
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!({$($map)*})] $($rest)*)
    };
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!($next),] $($rest)*)
    };
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::yaml_internal!(@array [$($elems,)* $crate::yaml_internal!($last)])
    };
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::yaml_internal!(@array [$($elems,)*] $($rest)*)
    };
    (@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        $crate::yaml_unexpected!($unexpected)
    };

    // Object parser
    (@object $object:ident () () ()) => {};
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($key)+).into(), $value);
        $crate::yaml_internal!(@object $object () ($($rest)*) ($($rest)*));
    };
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::yaml_unexpected!($unexpected);
    };
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($key)+).into(), $value);
    };
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!(null)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!(true)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!(false)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!([$($array)*])) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!({$($map)*})) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!($value)) , $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::yaml_internal!(@object $object [$($key)+] ($crate::yaml_internal!($value)));
    };
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        $crate::yaml_internal!();
    };
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        $crate::yaml_internal!();
    };
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        $crate::yaml_unexpected!($colon);
    };
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        $crate::yaml_unexpected!($comma);
    };
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::yaml_expect_expr_comma!($($unexpected)+);
    };
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::yaml_internal!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    // Main entry
    (null) => {
        $crate::Value::Null(None)
    };
    (true) => {
        $crate::Value::Bool(true, None)
    };
    (false) => {
        $crate::Value::Bool(false, None)
    };
    ([]) => {
        $crate::Value::Sequence($crate::Sequence { anchor: None, elements: $crate::__private::vec![] })
    };
    ([ $($tt:tt)+ ]) => {
        $crate::Value::Sequence($crate::Sequence { anchor: None, elements: $crate::yaml_internal!(@array [] $($tt)+) })
    };
    ({}) => {
        $crate::Value::Mapping($crate::Mapping::new())
    };
    ({ $($tt:tt)+ }) => {
        $crate::Value::Mapping({
            let mut object = $crate::Mapping::new();
            $crate::yaml_internal!(@object object () ($($tt)+) ($($tt)+));
            object
        })
    };
    ($other:expr) => {
        $crate::to_value(&$other).unwrap()
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! yaml_internal_vec {
    ($($content:tt)*) => {
        vec![$($content)*]
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! yaml_unexpected {
    () => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! yaml_expect_expr_comma {
    ($e:expr , $($tt:tt)*) => {};
}
