use std::io::Read;
use std::io::BufRead;
use std::io;
use std::cmp;

pub struct FillReader<R> {
    inner: R,
    buf: Box<[u8]>,
    pos: usize,
    cap: usize,
    overage: usize,
}

impl<R> FillReader<R> {
    fn new(inner: R, cap: usize, overage: usize) -> FillReader<R> {
        let len = cap.checked_add(overage).unwrap();
        FillReader {
            inner,
            buf: vec![0u8; len].into_boxed_slice(),
            pos: 0,
            cap: 0,
            overage,
        }
    }
}

impl<R: Read> FillReader<R> {
    pub fn fill(&mut self, amt: usize) -> io::Result<&[u8]> {
        assert_le!(amt, self.overage);
        assert_le!(self.pos, self.buf.len() - self.overage);

        loop {
            let found = self.inner.read(&mut self.buf[self.cap..])?;
            if 0 == found {
                break;
            }
            self.cap += found;

            if self.cap - self.pos >= amt {
                break;
            }
        }

        Ok(&self.buf)
    }
}

impl<R: Read> Read for FillReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.pos == self.cap && buf.len() >= self.buf.len() {
            return self.inner.read(buf);
        }
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}

impl<R: Read> BufRead for FillReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // If we've reached the end of our internal buffer then we need to fetch
        // some more data from the underlying reader.
        if self.pos == self.cap {
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.cap);
    }
}
