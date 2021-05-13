use std::io::{Error, ErrorKind};

use rayon::ThreadPoolBuildError;

pub struct MyThreadPoolBuildError(ThreadPoolBuildError);

impl From<MyThreadPoolBuildError> for Error {
    fn from(err: MyThreadPoolBuildError) -> Self {
        Error::new(ErrorKind::Other, err.0.to_string())
    }
}
