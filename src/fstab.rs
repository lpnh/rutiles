use libc::{FILE, c_char, c_int, endmntent, getmntent_r, mntent, setmntent};

use std::{
    ffi::{CStr, CString},
    io::{Error, ErrorKind, Result},
    ptr,
};
use tracing::debug;

// A single entry from `/etc/fstab`, based on the `mntent` struct:
// From `getmntent` man page:
//     > The mntent structure is defined in <mntent.h> as follows:
//     >      struct mntent {
//     >          char *mnt_fsname;   /* name of mounted filesystem */
//     >          char *mnt_dir;      /* filesystem path prefix */
//     >          char *mnt_type;     /* mount type (see mntent.h) */
//     >          char *mnt_opts;     /* mount options (see mntent.h) */
//     >          int   mnt_freq;     /* dump frequency in days */
//     >          int   mnt_passno;   /* pass number on parallel fsck */
//     >      };
#[derive(Debug)]
pub struct Fstab {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub options: Vec<String>,
    pub dump_freq: i32,
    pub fsck_pass: i32,
}

impl Fstab {
    unsafe fn from_mntent(ent: &mntent) -> Self {
        let device = unsafe { string_from_ptr(ent.mnt_fsname) };
        let mount_point = unsafe { string_from_ptr(ent.mnt_dir) };
        let fs_type = unsafe { string_from_ptr(ent.mnt_type) };
        let options = unsafe {
            string_from_ptr(ent.mnt_opts)
                .split(',')
                .map(String::from)
                .collect()
        };

        Self {
            device,
            mount_point,
            fs_type,
            options,
            dump_freq: ent.mnt_freq,
            fsck_pass: ent.mnt_passno,
        }
    }
}

// Pack `/etc/fstab` information
#[derive(Debug)]
pub struct FstabInfo {
    pub info: Vec<Fstab>,
}

// From the `fstab` man page:
// > The proper way to read records from fstab is to use the routines getmntent(3) or libmount
impl FstabInfo {
    pub fn new() -> Result<Self> {
        let file = FileHandle::new("/etc/fstab", "r")?;
        let mut info = Vec::new();

        // getmntent_r requires a buffer
        let buf_size: usize = 4096;
        let mut buf = vec![0 as c_char; buf_size];
        let mut ent = mntent {
            mnt_fsname: ptr::null_mut(),
            mnt_dir: ptr::null_mut(),
            mnt_type: ptr::null_mut(),
            mnt_opts: ptr::null_mut(),
            mnt_freq: 0,
            mnt_passno: 0,
        };

        // Unsafe safe: while not null
        while !unsafe { getmntent_r(file.0, &mut ent, buf.as_mut_ptr(), buf_size as c_int) }
            .is_null()
        {
            let entry = unsafe { Fstab::from_mntent(&ent) };
            debug!("Successfully parsed `fstab` entry for {}", entry.device);
            info.push(entry);
        }

        Ok(Self { info })
    }
}

// RAII wrapper for FILE pointer
struct FileHandle(*mut FILE);

impl FileHandle {
    fn new(path: &str, mode: &str) -> Result<Self> {
        let path = CString::new(path).map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
        let mode = CString::new(mode).map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;

        let file_ptr = unsafe { setmntent(path.as_ptr(), mode.as_ptr()) };
        if file_ptr.is_null() {
            return Err(Error::last_os_error());
        }

        debug!("Successfully opened `{}`", path.to_string_lossy());
        Ok(Self(file_ptr))
    }
}

// Use `endmntent` to close the stream
impl Drop for FileHandle {
    fn drop(&mut self) {
        unsafe { endmntent(self.0) };
    }
}

// Convert raw C string pointer to Rust String
unsafe fn string_from_ptr(ptr: *const i8) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let c_str = unsafe { CStr::from_ptr(ptr) };
    c_str.to_string_lossy().into_owned()
}
