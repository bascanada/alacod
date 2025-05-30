#[macro_export]
macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key.to_string(), $value);
            )+
            m
        }
     };
);

#[macro_export]
macro_rules! bmap(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::bevy::platform::collections::hash_map::HashMap::new();
            $(
                m.insert($key.to_string(), $value);
            )+
            m
        }
     };
);
