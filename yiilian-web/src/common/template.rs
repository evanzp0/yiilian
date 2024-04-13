
#[macro_export]
macro_rules! render {

    ($tpl_path: expr, {$($name: expr => $val: expr),* $(,)*}) => {
        {
            let mut context = tera::Context::new();
            $(
                context.insert($name, &$val);
            )*

            crate::common::app_state().tera().render($tpl_path, &context)
        }
    };

    ($tpl_path: expr, $val: ident) => {
        {
            let mut context = tera::Context::new();
            context.insert("value", &$val);
            crate::common::app_state().tera().render($tpl_path, &context)
        }
    };

    ($tpl_path: expr, $val: expr) => {
        {
            let mut context = tera::Context::new();
            context.insert("value", &$val);
            crate::common::app_state().tera().render($tpl_path, &context)
        }
    };

    ($tpl_path: expr) => {
        {
            let context = tera::Context::new();
            crate::common::app_state().tera().render($tpl_path, &context)
        }
    };
}