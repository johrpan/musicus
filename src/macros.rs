/// Simplification for pushing new screens.
///
/// This macro can be invoked in two forms.
///
/// 1. To push screens without an input value:
///
/// ```
/// let result = push!(handle, ScreenType).await;
/// ```
///
/// 2. To push screens with an input value:
///
/// ```
/// let result = push!(handle, ScreenType, input).await;
/// ```
#[macro_export]
macro_rules! push {
    ($handle:expr, $screen:ty) => {
        $handle.push::<_, _, $screen>(())
    };
    ($handle:expr, $screen:ty, $input:ident) => {
        $handle.push::<_, _, $screen>($input)
    };
}
