use self::platform::*;

use std::io::{self, Write};

mod platform;

pub trait AsRawObject {
   fn as_raw_object(&self) -> RawObject;
}

pub trait FromRawObject : Sized {
   unsafe fn from_raw_object(obj: RawObject) -> Option<Self>;
}

// TODO: also make a SpliceWriter that takes an input fd and and output fd and uses splice() to
//       transfer data
// TODO: make a TeeWriter or something that takes an input fd and two output fds and uses tee() to
//       transfer to both output fds

enum InnerZeroCopyWriter<T: Write + Sized> {
   Platform(PlatformZeroCopyWriter),
   Standard(T),
}

impl<T: Write + Sized> Write for InnerZeroCopyWriter<T> {
   fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      match self {
         InnerZeroCopyWriter::Platform(ref mut writer) => writer.write(buf),
         InnerZeroCopyWriter::Standard(ref mut writer) => writer.write(buf),
      }
   }

   fn flush(&mut self) -> io::Result<()> {
      match self {
         InnerZeroCopyWriter::Platform(ref mut writer) => writer.flush(),
         InnerZeroCopyWriter::Standard(ref mut writer) => writer.flush(),
      }
   }
}

pub struct ZeroCopyWriter<T: Write + AsRawObject + Sized> {
   /// This field is never used, but we need it to drop file descriptors
   #[allow(dead_code)]
   raw_obj_owner: Option<T>,

   inner: InnerZeroCopyWriter<T>,
}

struct TransformContainer<'a, A: Write + AsRawObject + Sized, B: Write + Sized> {
   /// This field is never used and probably could be converted into PhantomData, but might be
   /// useful for restructuring later (at the moment it's basically left over from an earlier
   /// design)
   #[allow(dead_code)]
   original: Option<&'a mut A>,

   transformed: Option<B>,
}

impl<'a, A: Write + AsRawObject + Sized, B: Write + Sized> Write for TransformContainer<'a, A, B> {
   fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
      self.transformed.as_mut().unwrap().write(bytes)
   }

   fn flush(&mut self) -> io::Result<()> {
      self.transformed.as_mut().unwrap().flush()
   }
}

impl<'a, A: Write + AsRawObject + Sized, B: Write + Sized> AsRawObject for TransformContainer<'a, A, B> {
   fn as_raw_object(&self) -> RawObject {
      panic!("Test should never be used")
   }
}

impl<T: Write + AsRawObject + Sized> ZeroCopyWriter<T> {
   pub fn new(writer: T) -> Self {
      let raw_obj = writer.as_raw_object();
      match unsafe { PlatformZeroCopyWriter::new(raw_obj) } {
         Ok(inner) => {
            ZeroCopyWriter {
               raw_obj_owner: Some(writer),
               inner: InnerZeroCopyWriter::Platform(inner),
            }
         }
         _ => {
            // creating the splice writer failed for whatever reason, so just make a default
            // writer
            ZeroCopyWriter {
               raw_obj_owner: None,
               inner: InnerZeroCopyWriter::Standard(writer),
            }
         }
      }
   }

   pub fn with_default<'a: 'b, 'b, F, W>(writer: &'a mut T, func: F) -> ZeroCopyWriter<impl Write + AsRawObject + Sized + 'b>
   where
      F: Fn(&'a mut T) -> W,
      W: Write + Sized + 'b,
   {
      let raw_obj = writer.as_raw_object();
      match unsafe { PlatformZeroCopyWriter::new(raw_obj) } {
         Ok(inner) => {
            ZeroCopyWriter {
               raw_obj_owner: Some(TransformContainer { original: Some(writer), transformed: None, }),
               inner: InnerZeroCopyWriter::Platform(inner),
            }
         }
         _ => {
            // XXX: should func actually consume writer and leave it up to the user to save the value?
            //      maybe provide a default stdin method then?  in some cases it would make more sense for the
            //      value to be consumed
            let real_writer = func(writer);
            ZeroCopyWriter {
               raw_obj_owner: None,
               inner: InnerZeroCopyWriter::Standard(TransformContainer { original: None, transformed: Some(real_writer) }),
            }
         }
      }
   }

   // XXX: unsure how to get something like this working without allocating, so not providing it
   /*pub fn stdout() -> ZeroCopyWriter<impl Write + AsRawObject + Sized> {
      let mut stdout = io::stdout();
      ZeroCopyWriter::with_default(&mut stdout, |stdout| {
         stdout.lock()
      })
   }*/
}

impl<T: Write + AsRawObject + Sized> Write for ZeroCopyWriter<T> {
   fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
      self.inner.write(buf)
   }

   fn flush(&mut self) -> io::Result<()> {
      self.inner.flush()
   }
}
