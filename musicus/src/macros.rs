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
    ($handle:expr, $screen:ty, $input:expr) => {
        $handle.push::<_, _, $screen>($input)
    };
}

/// Simplification for replacing the current navigator screen.
///
/// This macro can be invoked in two forms.
///
/// 1. To replace with screens without an input value:
///
/// ```
/// let result = replace!(navigator, ScreenType).await;
/// ```
///
/// 2. To replace with screens with an input value:
///
/// ```
/// let result = replace!(navigator, ScreenType, input).await;
/// ```
#[macro_export]
macro_rules! replace {
    ($navigator:expr, $screen:ty) => {
        $navigator.replace::<_, _, $screen>(())
    };
    ($navigator:expr, $screen:ty, $input:expr) => {
        $navigator.replace::<_, _, $screen>($input)
    };
}

/// Spawn a future on the GLib MainContext.
///
/// This can be invoked in the following forms:
///
/// 1. For spawning a future and nothing more:
///
/// ```
/// spawn!(async {
///     // Some code
/// });
///
/// 2. For spawning a future and cloning some data, that will be accessible
///    from the async code:
///
/// ```
/// spawn!(@clone data: Rc<_>, async move {
///     // Some code
/// });
#[macro_export]
macro_rules! spawn {
    ($future:expr) => {{
        let context = glib::MainContext::default();
        context.spawn_local($future);
    }};
    (@clone $data:ident, $future:expr) => {{
        let context = glib::MainContext::default();
        let $data = Rc::clone(&$data);
        context.spawn_local($future);
    }};
}
