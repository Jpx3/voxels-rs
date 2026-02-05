use io::{Error, ErrorKind};
use robusta_jni::jni::{JNIEnv, JavaVM};
use robusta_jni::jni::objects::{GlobalRef, JObject, JValue};
use std::io::{self, Read, Write};

pub struct JavaInputStream {
    vm: JavaVM,
    stream: GlobalRef,
}

pub struct JavaOutputStream {
    vm: JavaVM,
    stream: GlobalRef,
}

impl JavaInputStream {
    pub fn new(env: &JNIEnv, stream: JObject) -> Result<Self, robusta_jni::jni::errors::Error> {
        Ok(Self {
            vm: env.get_java_vm()?,
            stream: env.new_global_ref(stream)?,
        })
    }
}

impl JavaOutputStream {
    pub fn new(env: &JNIEnv, stream: JObject) -> Result<Self, robusta_jni::jni::errors::Error> {
        Ok(Self {
            vm: env.get_java_vm()?,
            stream: env.new_global_ref(stream)?,
        })
    }
}

impl Read for JavaInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let env = self.vm.attach_current_thread()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        let java_array = env.new_byte_array(buf.len() as i32)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        let read_result = env.call_method(
            self.stream.as_obj(),
            "read",
            "([B)I",
            &[JValue::Object(java_array.into())],
        );
        let bytes_read_jint = match read_result {
            Ok(val) => val.i().unwrap_or(-1),
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };
        if bytes_read_jint == -1 {
            return Ok(0);
        }
        let bytes_read = bytes_read_jint as usize;
        let mut internal_buf = vec![0i8; bytes_read];
        env.get_byte_array_region(java_array, 0, &mut internal_buf)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        for (i, &val) in internal_buf.iter().enumerate() {
            buf[i] = val as u8;
        }
        Ok(bytes_read)
    }
}

impl Write for JavaOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let env = self.vm.attach_current_thread()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let java_array = env.new_byte_array(buf.len() as i32)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let internal_buf: &[i8] = unsafe { std::mem::transmute(buf) };

        env.set_byte_array_region(java_array, 0, internal_buf)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        env.call_method(
            self.stream.as_obj(),
            "write",
            "([B)V",
            &[JValue::Object(java_array.into())],
        ).map_err(|e| Error::new(ErrorKind::Other, e))?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let env = self.vm.attach_current_thread()
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        env.call_method(self.stream.as_obj(), "flush", "()V", &[])
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        Ok(())
    }
}