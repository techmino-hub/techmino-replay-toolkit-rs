/// Defines "easy" getters and setters for a `HashMap<String, Value>`-based
/// struct.
macro_rules! metadata_getters_setters {
    //////////////////// ENTRY POINT ////////////////////
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
    //////////////////// NORMALIZATION ////////////////////
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
    //////////////////// CORE ////////////////////
    (
        @fndef($innername:ident);
        $(#[$attr:meta])*
        $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define key for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get_or_raw for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get_raw for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get_raw_mut for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define set for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define remove for $key $name : $type | $ownedtype where {
                innername: $innername,
                from_json: $from_json,
                to_json: $to_json
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
        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define key for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get_or_raw for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get_raw for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
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

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define get_raw_mut for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define set for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
            }
        }

        $crate::macros::getters_setters_defs! {
            $(#[$attr])*
            define remove for $key $name : $type | $ownedtype where {
                innername: $innername ,
                from_json: $from_json ,
                to_json: $to_json
            }
        }
    };
}

/// Common functions shared between the metadata and settings getters/setters.
macro_rules! getters_setters_defs {
    (
        $(#[$attr:meta])*
        define key for $key:literal $name:ident $( : $_type:ty | $_ownedtype:ty where {
            innername: $_innername:ident ,
            from_json: $_from_json:expr ,
            to_json: $_to_json:expr
        })?
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
        }
    };
    (
        $(#[$attr:meta])*
        define get for $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            innername: $innername:ident ,
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            $(#[$attr])*
            ///
            /// # Strict Getter
            /// Strict getter methods attempt to convert the stored value into
            /// the normal strictly-typed version for convenience.
            ///
            /// # Errors
            /// The [`TypeError`] struct contains a reference to the raw
            /// [`serde_json::Value`] value if you need it.
            /// Alternatively, use the shortcut method `get_*_or_raw()` if you
            /// don't need the error reason.
            #[must_use]
            pub fn [<get_ $name>](&self) ->
                ::core::option::Option<::core::result::Result<$type, $crate::types::TypeError<'_>>>
            {
                let value: &::serde_json::Value = self.$innername.get(Self::[<KEY_ $name:snake:upper>])?;

                let retval: ::core::result::Result<$type, $crate::types::ValueVariant> =
                    $from_json(value);

                let retval = match retval {
                    Ok(v) => v,
                    Err(exp_ty) => return Some(Err($crate::types::TypeError {
                        key: Self::[<KEY_ $name:snake:upper>],
                        exp_ty,
                        value,
                    }))
                };

                Some(Ok(retval))
            }
        }
    };
    (
        $(#[$attr:meta])*
        define get_or_raw for $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            innername: $innername:ident ,
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            $(#[$attr])*
            ///
            /// # Strict-or-raw Getter
            /// Strict-or-raw getter methods attempt to convert the stored value
            /// into the normal strictly-typed version for convenience.
            ///
            /// # Errors
            /// Returns the raw [`serde_json::Value`] if conversion fails.
            /// Additional error info (e.g. the expected type) is omitted in
            /// this method, if you need it, use the `get_*()` methods instead.
            #[must_use]
            pub fn [<get_ $name _or_raw>](&self) ->
                ::core::option::Option<::core::result::Result<$type, &::serde_json::Value>>
            {
                self.[<get_ $name>]().map(|res| res.map_err(|err| err.inner()))
            }
        }
    };
    (
        $(#[$attr:meta])*
        define get_raw for $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            innername: $innername:ident ,
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            $(#[$attr])*
            ///
            /// # Raw Getter
            /// Raw getter methods do *not* try to convert the stored value to
            /// the normal strictly-typed version.
            #[must_use]
            pub fn [<get_ $name _raw>](&self) ->
                ::core::option::Option<&::serde_json::Value>
            {
                self.$innername.get(Self::[<KEY_ $name:snake:upper>])
            }
        }
    };
    (
        $(#[$attr:meta])*
        define get_raw_mut for $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            innername: $innername:ident ,
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            $(#[$attr])*
            ///
            /// # Raw Getter
            /// Raw getter methods do *not* try to convert the stored value to
            /// the normal strictly-typed version.
            #[must_use]
            pub fn [<get_ $name _raw_mut>](&mut self) ->
                ::core::option::Option<&mut ::serde_json::Value>
            {
                self.$innername.get_mut(Self::[<KEY_ $name:snake:upper>])
            }
        }
    };
    (
        $(#[$attr:meta])*
        define set for $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            innername: $innername:ident ,
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            $(#[$attr])*
            ///
            /// # Returns
            /// Returns the old value of the field.
            /// - If there was no old value of the field, returns `None`.
            /// - If conversion succeeds, returns the strictly typed version
            ///   (`Some(Ok(T))`).
            /// - Otherwise, returns the error that happened while attempting to
            ///   convert the value.
            ///   (`Some(Err(OwnedTypeError)))`)
            ///   - You can then try to get the inner value using
            ///   [`.inner()`][crate::types::OwnedTypeError::inner]
            ///
            /// # Errors
            /// Converting the old stored value into a strict form may fail.
            ///
            /// When this happens, the map is still set properly.
            ///
            /// You can then display the error using its `Display` impl or try
            /// to get the inner value using
            /// [`.inner()`][crate::types::OwnedTypeError::inner].
            pub fn [<set_ $name>](
                &mut self,
                value: $type
            ) -> ::core::option::Option<
                ::core::result::Result<$ownedtype, $crate::types::OwnedTypeError>
            > {
                let value: ::serde_json::Value = $to_json(value);

                if let Some(mutref) = self.$innername.get_mut(Self::[<KEY_ $name:snake:upper>]) {
                    let json: ::serde_json::Value = ::core::mem::replace(mutref, value);
                    return match $from_json(&json) {
                        Ok(v) => Some(Ok(v.into())),
                        Err(exp_ty) => Some(Err($crate::types::OwnedTypeError {
                            key: Self::[<KEY_ $name:snake:upper>],
                            exp_ty,
                            value: json,
                        }))
                    }
                }

                let json: ::serde_json::Value = self.$innername
                    .insert(Self::[<KEY_ $name:snake:upper>].to_owned(), value)?;
                match $from_json(&json) {
                    Ok(v) => Some(Ok(v.into())),
                    Err(exp_ty) => Some(Err($crate::types::OwnedTypeError {
                        key: Self::[<KEY_ $name:snake:upper>],
                        exp_ty,
                        value: json,
                    }))
                }
            }
        }
    };
    (
        $(#[$attr:meta])*
        define remove for $key:literal $name:ident : $type:ty | $ownedtype:ty where {
            innername: $innername:ident ,
            from_json: $from_json:expr ,
            to_json: $to_json:expr
        }
    ) => {
        ::pastey::paste! {
            $(#[$attr])*
            ///
            /// # Returns
            /// Returns the old value of the field.
            /// - If there was no old value of the field, returns `None`.
            /// - If conversion succeeds, returns the strictly typed version
            ///   (`Some(Ok(T))`).
            /// - Otherwise, returns the error that happened while attempting to
            ///   convert the value.
            ///   (`Some(Err(OwnedTypeError)))`)
            ///   - You can then try to get the inner value using
            ///   [`.inner()`][crate::types::OwnedTypeError::inner]
            ///
            /// # Errors
            /// Converting the old stored value into a strict form may fail.
            ///
            /// When this happens, the map is still set properly.
            ///
            /// You can then display the error using its `Display` impl or try
            /// to get the inner value using
            /// [`.inner()`][crate::types::OwnedTypeError::inner].
            pub fn [<remove_ $name>](&mut self) -> ::core::option::Option<
                ::core::result::Result<$ownedtype, $crate::types::OwnedTypeError>
            > {
                let json = self.$innername.remove(Self::[<KEY_ $name:snake:upper>])?;
                let res: ::core::result::Result<$type, $crate::types::ValueVariant> =
                    $from_json(&json);

                match res {
                    Ok(processed) => {
                        let processed: $type = processed;
                        let owned: $ownedtype = processed.into();

                        return Some(Ok(owned));
                    }
                    Err(exp_ty) => {
                        return Some(Err($crate::types::OwnedTypeError {
                            key: Self::[<KEY_ $name:snake:upper>],
                            exp_ty,
                            value: json,
                        }));
                    }
                }
            }
        }
    };
}

pub(crate) use getters_setters_defs;
pub(crate) use metadata_getters_setters;
pub(crate) use setting_getters_setters;
