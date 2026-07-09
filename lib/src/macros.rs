/// Defines "easy" getters and setters for a `HashMap<String, Value>`-based
/// struct.
macro_rules! metadata_getters_setters {
    (
        ($innername:ident);
        $(
            $(#[$attr:meta])*
            $key:literal $name:ident : $type:ty $( | $ownedtype:ty )? $(where {
                $($convkey:ident : $convval:expr),* $(,)?
            })?
        ),* $(,)?
    ) => {
        $(
            metadata_getters_setters! {
                @resolvetype($innername);
                $(#[$attr])*
                $key $name : $type $( | $ownedtype )? $(where {
                    $($convkey: $convval,)*
                })?
            }
        )*
    };
    (
        @resolvetype($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty $(where {
            $($convkey:ident : $convval:expr),* $(,)?
        })?
    ) => {
        metadata_getters_setters! {
            @resolveconv($innername);
            $(#[$attr])*
            $key $name : $type | $type $(where {
                $($convkey: $convval,)*
            })?
        }
    };
    (
        @resolvetype($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty $(where {
            $($convkey:ident : $convval:expr),* $(,)?
        })?
    ) => {
        metadata_getters_setters! {
            @resolveconv($innername);
            $(#[$attr])*
            $key $name : $type | $ownedtype $(where {
                $($convkey: $convval,)*
            })?
        }
    };
    (
        @resolveconv($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty $(where { $(,)* })?
    ) => {
        metadata_getters_setters! {
            @fndef($innername);
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: (|value: &::serde_json::Value| value.try_into().ok()),
                to_json: core::convert::Into::into,
            }
        }
    };
    (
        @resolveconv($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            to_json: $to_json:expr $(,)*
        }
    ) => {
        metadata_getters_setters! {
            @fndef($innername);
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: (|value: &::serde_json::Value| value.try_into().ok()),
                to_json: $to_json
            }
        }
    };
    (
        @resolveconv($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr $(,)*
        }
    ) => {
        metadata_getters_setters! {
            @fndef($innername);
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: core::convert::Into::into
            }
        }
    };
    (
        @resolveconv($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            to_json: $to_json:expr ,
            from_json: $from_json:expr $(,)*
        }
    ) => {
        metadata_getters_setters! {
            @fndef($innername);
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: $to_json
            }
        }
    };
    (
        @resolveconv($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr $(,)*
        }
    ) => {
        metadata_getters_setters! {
            @fndef($innername);
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: $to_json
            }
        }
    };
    (
        @fndef($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            #[doc = concat!(
                "Gets the key for the `",
                stringify!($name),
                "` entry in the map (is currently ",
                stringify!($key),
                ")."
            )]
            ///
            /// This is useful for manually indexing into the map to get a
            /// specific entry. However, usually, the default `get_*` or
            /// `set_*` methods should be enough for almost all usecases.
            pub const [<KEY_ $name:snake:upper>]: &str = $key;

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name>](&self) ->
                ::core::option::Option<::core::result::Result<$type, $crate::types::TypeError>>
            {
                let value: &::serde_json::Value = self.$innername.get(Self::[<KEY_ $name:snake:upper>])?;

                let retval: ::core::option::Option<$type> = $from_json(value);
                let retval = retval.ok_or($crate::types::TypeError(()));

                Some(retval)
            }

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name _or_raw>](&self) ->
                ::core::option::Option<::core::result::Result<$type, &::serde_json::Value>>
            {
                let value: &::serde_json::Value = self.$innername.get(Self::[<KEY_ $name:snake:upper>])?;

                let retval: ::core::option::Option<$type> = $from_json(value);
                let retval = retval.ok_or(value);

                Some(retval)
            }

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name _raw>](&self) ->
                ::core::option::Option<&::serde_json::Value>
            {
                self.$innername.get(Self::[<KEY_ $name:snake:upper>])
            }

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name _raw_mut>](&mut self) ->
                ::core::option::Option<&mut ::serde_json::Value>
            {
                self.$innername.get_mut(Self::[<KEY_ $name:snake:upper>])
            }

            $(#[$attr])*
            ///
            /// # Returns
            /// Returns the old value of the field.
            /// - If there was no old value of the field, returns `None`.
            /// - If conversion succeeds, returns the strictly typed version
            ///   (`Some(Ok(T))`).
            /// - Otherwise, returns the raw JSON value.
            ///   (`Some(Err(serde_json::Value)))`)
            pub fn [<set_ $name>](&mut self, value: ::core::option::Option<$type>) ->
                Option<Result<$ownedtype, ::serde_json::Value>>
            {
                let Some(value) = value else {
                    let json = self.$innername.remove(Self::[<KEY_ $name:snake:upper>])?;
                    if let Some(processed) = $from_json(&json) {
                        let processed: $type = processed;
                        let owned: $ownedtype = processed.into();

                        return Some(Ok(owned));
                    }

                    return Some(Err(json));
                };

                let value: ::serde_json::Value = $to_json(value);

                if let Some(mutref) = self.$innername.get_mut(Self::[<KEY_ $name:snake:upper>]) {
                    let json = ::core::mem::replace(mutref, value);
                    return Some($from_json(&json).map(|v| v.into()).ok_or(json));
                }

                let json = self.$innername.insert(Self::[<KEY_ $name:snake:upper>].to_owned(), value)?;
                return Some($from_json(&json).map(|v| v.into()).ok_or(json));
            }
        }
    };
}

/// Defines "easy" getters and setters for a struct based on an immutable
/// or mutable reference of a `serde_json::Map<String, Value>`.
macro_rules! setting_getters_setters {
    (
        {
            owned_struct: $ownedcontainer:ty => $ownedinner:ident,
            ref_struct: $refcontainer:ty => $refinner:ident,
            mut_ref_struct: $mutcontainer:ty => $mutinner:ident,
        };
        $(
            $(#[$attr:meta])*
            $key:literal $name:ident : $type:ty $( | $ownedtype:ty )? $(where {
                $($convkey:ident : $convval:expr),* $(,)?
            })?
        ),* $(,)?
    ) => {
        impl $ownedcontainer {
            $(
                setting_getters_setters! {
                    @resolvetype {
                        kind: "owned",
                        inner: $ownedinner
                    };
                    $(#[$attr])*
                    $key $name : $type $( | $ownedtype )? $(where {
                        $($convkey: $convval,)*
                    })?
                }
            )*
        }
        impl $refcontainer {
            $(
                setting_getters_setters! {
                    @resolvetype {
                        kind: "ref",
                        inner: $refinner
                    };
                    $(#[$attr])*
                    $key $name : $type $( | $ownedtype )? $(where {
                        $($convkey: $convval,)*
                    })?
                }
            )*
        }
        impl $mutcontainer {
            $(
                setting_getters_setters! {
                    @resolvetype {
                        kind: "ref mut",
                        inner: $mutinner
                    };
                    $(#[$attr])*
                    $key $name : $type $( | $ownedtype )? $(where {
                        $($convkey: $convval,)*
                    })?
                }
            )*
        }
    };
    (
        @resolvetype {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty $(where {
            $($convkey:ident : $convval:expr),* $(,)?
        })?
    ) => {
        setting_getters_setters! {
            @resolveconv {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $type $(where {
                $($convkey: $convval,)*
            })?
        }
    };
    (
        @resolvetype {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty $(where {
            $($convkey:ident : $convval:expr),* $(,)?
        })?
    ) => {
        setting_getters_setters! {
            @resolveconv {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype $(where {
                $($convkey: $convval,)*
            })?
        }
    };
    (
        @resolveconv {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty $(where { $(,)* })?
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: (|value: &::serde_json::Value| value.try_into().ok()),
                to_json: core::convert::Into::into,
            }
        }
    };
    (
        @resolveconv {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            to_json: $to_json:expr $(,)*
        }
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: (|value: &::serde_json::Value| value.try_into().ok()),
                to_json: $to_json
            }
        }
    };
    (
        @resolveconv {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr $(,)*
        }
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: core::convert::Into::into
            }
        }
    };
    (
        @resolveconv {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            to_json: $to_json:expr ,
            from_json: $from_json:expr $(,)*
        }
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: $to_json
            }
        }
    };
    (
        @resolveconv {
            kind: $kind:tt,
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr $(,)*
        }
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: $kind,
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: $to_json
            }
        }
    };
    (
        @fndef {
            kind: "owned",
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: "ref mut",
                inner: $innername
            };

            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: $to_json
            }
        }
    };
    (
        @fndef {
            kind: "ref",
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            #[doc = concat!(
                "Gets the key for the `",
                stringify!($name),
                "` entry in the map (is currently ",
                stringify!($key),
                ")."
            )]
            ///
            /// This is useful for manually indexing into the map to get a
            /// specific entry. However, usually, the default `get_*` or
            /// `set_*` methods should be enough for almost all usecases.
            pub const [<KEY_ $name:snake:upper>]: &'static str = $key;

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name>](&self) ->
                ::core::option::Option<::core::result::Result<$type, $crate::types::TypeError>>
            {
                let value: &::serde_json::Value = self.$innername.get(Self::[<KEY_ $name:snake:upper>])?;

                let retval: ::core::option::Option<$type> = $from_json(value);
                let retval = retval.ok_or($crate::types::TypeError(()));

                Some(retval)
            }

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name _or_raw>](&self) ->
                ::core::option::Option<::core::result::Result<$type, &::serde_json::Value>>
            {
                let value: &::serde_json::Value = self.$innername.get(Self::[<KEY_ $name:snake:upper>])?;

                let retval: ::core::option::Option<$type> = $from_json(value);
                let retval = retval.ok_or(value);

                Some(retval)
            }

            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name _raw>](&self) ->
                ::core::option::Option<&::serde_json::Value>
            {
                self.$innername.get(Self::[<KEY_ $name:snake:upper>])
            }
        }
    };
    (
        @fndef {
            kind: "ref mut",
            inner: $innername:ident
        };
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        setting_getters_setters! {
            @fndef {
                kind: "ref",
                inner: $innername
            };
            $(#[$attr])*
            $key $name : $type | $ownedtype where {
                from_json: $from_json ,
                to_json: $to_json
            }
        }

        ::pastey::paste! {
            $(#[$attr])*
            #[must_use]
            pub fn [<get_ $name _raw_mut>](&mut self) ->
                ::core::option::Option<&mut ::serde_json::Value>
            {
                self.$innername.get_mut(Self::[<KEY_ $name:snake:upper>])
            }

            $(#[$attr])*
            ///
            /// # Returns
            /// Returns the old value of the field.
            /// - If there was no old value of the field, returns `None`.
            /// - If conversion succeeds, returns the strictly typed version
            ///   (`Some(Ok(T))`).
            /// - Otherwise, returns the raw JSON value.
            ///   (`Some(Err(serde_json::Value)))`)
            pub fn [<set_ $name>](&mut self, value: ::core::option::Option<$type>) ->
                Option<Result<$ownedtype, ::serde_json::Value>>
            {
                let Some(value) = value else {
                    let json = self.$innername.remove(Self::[<KEY_ $name:snake:upper>])?;
                    if let Some(processed) = $from_json(&json) {
                        let processed: $type = processed;
                        let owned: $ownedtype = processed.into();

                        return Some(Ok(owned));
                    }

                    return Some(Err(json));
                };

                let value: ::serde_json::Value = $to_json(value);

                if let Some(mutref) = self.$innername.get_mut(Self::[<KEY_ $name:snake:upper>]) {
                    let json = ::core::mem::replace(mutref, value);
                    return Some($from_json(&json).map(|v| v.into()).ok_or(json));
                }

                let json = self.$innername.insert(<str as ::alloc::borrow::ToOwned>::to_owned(Self::[<KEY_ $name:snake:upper>]), value)?;
                return Some($from_json(&json).map(|v| v.into()).ok_or(json));
            }
        }
    };
}

pub(crate) use metadata_getters_setters;
pub(crate) use setting_getters_setters;
