#[macro_export]
macro_rules! context_trait {
    ($e:path) => {
        pub trait Context<T, E> {
            fn context<C>(self, cxt: C) -> std::result::Result<T, $e>
            where
                C: Into<String>;
        }
    };
}
