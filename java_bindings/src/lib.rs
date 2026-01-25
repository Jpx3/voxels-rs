use robusta_jni::bridge;
use robusta_jni::convert::{FromJavaValue, Signature, TryFromJavaValue, TryIntoJavaValue};
use robusta_jni::jni::errors::Error as JniError;
use robusta_jni::jni::errors::Result as JniResult;
use robusta_jni::jni::objects::{AutoLocal, JObject};
use robusta_jni::jni::JNIEnv;
use voxels_core::common::BlockPosition;

trait JNITranslation {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>>;

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self>
    where
        Self: Sized;
}

impl JNITranslation for BlockPosition {
    fn to_jni<'env>(&self, env: &JNIEnv<'env>) -> JniResult<JObject<'env>> {
        let class = env.find_class("de/richy/voxels/BlockPosition")?;
        let obj = env.new_object(
            class,
            "(III)V",
            &[
                self.x.into(),
                self.y.into(),
                self.z.into(),
            ],
        )?;
        Ok(obj)
    }

    fn from_jni<'env>(env: &JNIEnv<'env>, obj: JObject<'env>) -> JniResult<Self> {
        let x = env.get_field(obj, "x", "I")?.i()?;
        let y = env.get_field(obj, "y", "I")?.i()?;
        let z = env.get_field(obj, "z", "I")?.i()?;
        Ok(BlockPosition { x, y, z })
    }
}

#[bridge]
mod jni {
    use super::*;

    #[derive(Signature, TryIntoJavaValue, TryFromJavaValue, FromJavaValue)]
    #[package(de.richy.voxels)]
    pub struct Voxels<'env: 'borrow, 'borrow> {
        #[instance]
        raw: AutoLocal<'env, 'borrow>,
    }

    impl<'env: 'borrow, 'borrow> Voxels<'env, 'borrow> {
        pub extern "jni" fn blocksFromBytes(
            env: &JNIEnv<'env>,
            _input_stream: JObject<'env>,
            _schematic_type: JObject<'env>,
        ) -> JniResult<JObject<'env>> {
            env.new_object("de/richy/voxels/BlockInputStream", "()V", &[])
        }
    }

    #[package(de.richy.voxels)]
    pub struct BlockInputStream;

    impl BlockInputStream {
        pub extern "jni" fn read<'env>(
            env: &JNIEnv,
            _blocks_array: JObject<'env>,
            offset: i32,
            length: i32,
        ) -> JniResult<i32> {
            println!(
                "BlockInputStream read called with offset: {}, length: {}",
                offset, length
            );

            let bp = BlockPosition::new(0, 0, 0);
            let jni_obj = bp.to_jni(env)?;
            // println!("Converted BlockPosition to JNI object: {:?}", jni_obj);
            // call toString on it and print
            // let to_string_method_id = env.get_method_id("de/richy/voxels/BlockPosition", "toString", "()Ljava/lang/String;")?;
            let jni_str_obj = env.call_method(jni_obj, "toString", "()Ljava/lang/String;", &[])?.l()?;
            let jni_str: String = env.get_string(jni_str_obj.into())?.into();
            println!("BlockPosition JNI object toString: {}", jni_str);

            Ok(-1)
        }

        pub extern "jni" fn close(env: &JNIEnv) -> JniResult<()> {
            println!("BlockInputStream closed");
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use std::fs;
        use std::process::{Command, Stdio};

        #[test]
        fn test_java() {
            compile_java_test_class();

            let separator = if cfg!(target_os = "windows") {
                ";"
            } else {
                ":"
            };

            let java_classpath = &format!(
                "../target/voxels.jar{}../target/java_test_classes",
                separator
            );

            let status = Command::new("java")
                // inherit stdout and stderr
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .args([
                    "-cp",
                    java_classpath,
                    "-Djava.library.path=../target/release",
                    "TestAll",
                ])
                .status()
                .expect("Failed to run Java test class");

            if !status.success() {
                panic!("Java test class failed!");
            }
        }

        fn compile_java_test_class() {
            let java_src_dir = "java_test";
            let out_dir = "../target/java_test_classes";
            fs::create_dir_all(out_dir).unwrap();
            let status = Command::new("javac")
                .args([
                    "-d",
                    out_dir,
                    "-cp",
                    "../target/voxels.jar",
                    &format!("{}/TestAll.java", java_src_dir),
                ])
                .status()
                .expect("Failed to run javac");

            if !status.success() {
                panic!("Java test class compilation failed: {:?}", status);
            }
        }
    }
}
