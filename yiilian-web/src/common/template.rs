
#[macro_export]
macro_rules! render {
    ($tpl_path: expr, {$($name: expr => $val: expr),*}) => {
        {
            let mut context = tera::Context::new();
                
            $(
                context.insert($name, &$val);
            )*

            crate::common::app_state().tera().render($tpl_path, &context).unwrap()
        }
    };
}