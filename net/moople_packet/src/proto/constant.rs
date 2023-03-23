


#[macro_export]
macro_rules! maple_constant {
    ($name:ident, $ty:ty, $val:expr) => {
        #[derive(Debug, Default, Clone, Copy)]
        pub struct $name;

        impl $crate::proto::wrapped::MapleWrapped for $name {
            type Inner = $ty;

            fn maple_into_inner(&self) -> Self::Inner {
                $val
            }

            fn maple_from(_v: Self::Inner) -> Self {
                //TODO should the constant be verified?
                // maybe just in debug mode not sure yet
                /*if v != $val {
                    panic!("Invalid constant")
                }*/

                Self
            }
        }
    };
}

maple_constant!(Zero32, u32, 0);
