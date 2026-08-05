/// The name Android holds for a picked file.
///
/// The document picker hands back a `content://` URI. Most carry no filename anywhere in
/// the string — `content://…/document/audio%3A1000000123` is a row id — so the only place
/// the name a person recognises exists is the `_display_name` column the provider answers
/// with.
pub struct ContentUriName;

impl ContentUriName {
    const DISPLAY_NAME: &'static str = "_display_name";

    #[cfg(not(target_os = "android"))]
    pub fn resolve(_uri: &str) -> Option<String> {
        None
    }

    #[cfg(target_os = "android")]
    pub fn resolve(uri: &str) -> Option<String> {
        if !uri.starts_with("content://") {
            return None;
        }
        match Self::query(uri) {
            Ok(name) => name,
            Err(e) => {
                log::warn!("could not read the display name for {uri}: {e}");
                None
            }
        }
    }

    #[cfg(target_os = "android")]
    fn query(uri: &str) -> Result<Option<String>, anyhow::Error> {
        use anyhow::anyhow;
        use jni::JavaVM;
        use jni::objects::{JObject, JObjectArray, JString, JValue};

        let ctx = ndk_context::android_context();
        if ctx.vm().is_null() || ctx.context().is_null() {
            return Err(anyhow!("no android context"));
        }

        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

        let uri_string: JString = env.new_string(uri)?;
        let parsed = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&uri_string)],
            )?
            .l()?;

        let resolver = env
            .call_method(
                &activity,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )?
            .l()?;

        let column: JString = env.new_string(Self::DISPLAY_NAME)?;
        let projection: JObjectArray =
            env.new_object_array(1, "java/lang/String", JObject::null())?;
        env.set_object_array_element(&projection, 0, &column)?;

        let cursor = env
            .call_method(
                &resolver,
                "query",
                "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
                &[
                    JValue::Object(&parsed),
                    JValue::Object(&projection),
                    JValue::Object(&JObject::null()),
                    JValue::Object(&JObject::null()),
                    JValue::Object(&JObject::null()),
                ],
            )?
            .l()?;

        if cursor.is_null() {
            return Ok(None);
        }

        let name = Self::read_first(&mut env, &cursor);
        env.call_method(&cursor, "close", "()V", &[]).ok();
        name
    }

    #[cfg(target_os = "android")]
    fn read_first(
        env: &mut jni::JNIEnv,
        cursor: &jni::objects::JObject,
    ) -> Result<Option<String>, anyhow::Error> {
        use jni::objects::{JString, JValue};

        if !env.call_method(cursor, "moveToFirst", "()Z", &[])?.z()? {
            return Ok(None);
        }

        let column: JString = env.new_string(Self::DISPLAY_NAME)?;
        let index = env
            .call_method(
                cursor,
                "getColumnIndex",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&column)],
            )?
            .i()?;
        if index < 0 {
            return Ok(None);
        }

        let value = env
            .call_method(
                cursor,
                "getString",
                "(I)Ljava/lang/String;",
                &[JValue::Int(index)],
            )?
            .l()?;
        if value.is_null() {
            return Ok(None);
        }

        let name: String = env.get_string(&JString::from(value))?.into();
        Ok(Some(name).filter(|n| !n.trim().is_empty()))
    }
}
