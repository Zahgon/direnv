//! Go's `path/filepath` semantics for slash-separated paths.
//!
//! `Clean` is purely lexical — it never touches the filesystem — and direnv
//! relies on that: `FileTimes::new_time` canonicalises `docs/../file` without
//! `docs` having to exist, and the allow-store hashes are taken over the
//! lexically cleaned absolute path. `std::path` normalises differently (it
//! keeps `..` components), so the algorithm is reproduced here.

use std::io;
use std::path::Path;

const SEP: u8 = b'/';

/// Go's `filepath.Clean`.
pub fn clean(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let input = path.as_bytes();
    let rooted = input[0] == SEP;
    let n = input.len();

    let mut out: Vec<u8> = Vec::with_capacity(n);
    let mut r = 0usize;
    let mut dotdot = 0usize;
    if rooted {
        out.push(SEP);
        r = 1;
        dotdot = 1;
    }

    while r < n {
        if input[r] == SEP || (input[r] == b'.' && (r + 1 == n || input[r + 1] == SEP)) {
            // An empty element or a "." element: skip it.
            r += 1;
        } else if input[r] == b'.'
            && r + 1 < n
            && input[r + 1] == b'.'
            && (r + 2 == n || input[r + 2] == SEP)
        {
            r += 2;
            if out.len() > dotdot {
                // Walk back to the separator that starts the last element and
                // drop it too, exactly as Go's index-based loop does.
                let mut w = out.len() - 1;
                while w > dotdot && out[w] != SEP {
                    w -= 1;
                }
                out.truncate(w);
            } else if !rooted {
                if !out.is_empty() {
                    out.push(SEP);
                }
                out.push(b'.');
                out.push(b'.');
                dotdot = out.len();
            }
        } else {
            if (rooted && out.len() != 1) || (!rooted && !out.is_empty()) {
                out.push(SEP);
            }
            while r < n && input[r] != SEP {
                out.push(input[r]);
                r += 1;
            }
        }
    }

    if out.is_empty() {
        return ".".to_string();
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Go's `filepath.IsAbs`.
pub fn is_abs(path: &str) -> bool {
    path.starts_with('/')
}

/// Go's `filepath.Join` — join the non-empty elements with `/`, then `Clean`.
pub fn join(elems: &[&str]) -> String {
    let mut joined = String::new();
    for elem in elems {
        if elem.is_empty() {
            continue;
        }
        if joined.is_empty() {
            joined.push_str(elem);
        } else {
            joined.push('/');
            joined.push_str(elem);
        }
    }
    if joined.is_empty() {
        return String::new();
    }
    clean(&joined)
}

/// Go's `filepath.Base`.
pub fn base(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let mut bytes = path.as_bytes();
    while !bytes.is_empty() && bytes[bytes.len() - 1] == SEP {
        bytes = &bytes[..bytes.len() - 1];
    }
    if let Some(idx) = bytes.iter().rposition(|&b| b == SEP) {
        bytes = &bytes[idx + 1..];
    }
    if bytes.is_empty() {
        return "/".to_string();
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Go's `filepath.Dir`.
pub fn dir(path: &str) -> String {
    match path.as_bytes().iter().rposition(|&b| b == SEP) {
        Some(idx) => clean(&path[..idx + 1]),
        None => clean(""),
    }
}

/// Go's `filepath.Abs` — join with the process working directory, then `Clean`.
pub fn abs(path: &str) -> io::Result<String> {
    if is_abs(path) {
        return Ok(clean(path));
    }
    let wd = std::env::current_dir()?;
    let wd = wd.to_string_lossy().into_owned();
    Ok(join(&[&wd, path]))
}

/// Go's `filepath.Rel`.
pub fn rel(basepath: &str, targpath: &str) -> Result<String, String> {
    let base_clean = clean(basepath);
    let targ_clean = clean(targpath);
    if targ_clean == base_clean {
        return Ok(".".to_string());
    }
    let base = if base_clean == "." {
        ""
    } else {
        &base_clean[..]
    };
    let targ = &targ_clean[..];

    let base_slashed = base.starts_with('/');
    let targ_slashed = targ.starts_with('/');
    if base_slashed != targ_slashed {
        return Err(format!("Rel: can't make {targpath} relative to {basepath}"));
    }

    let bb = base.as_bytes();
    let tb = targ.as_bytes();
    let (bl, tl) = (bb.len(), tb.len());
    let (mut b0, mut bi, mut t0, mut ti) = (0usize, 0usize, 0usize, 0usize);
    loop {
        while bi < bl && bb[bi] != SEP {
            bi += 1;
        }
        while ti < tl && tb[ti] != SEP {
            ti += 1;
        }
        if tb[t0..ti] != bb[b0..bi] {
            break;
        }
        if bi < bl {
            bi += 1;
        }
        if ti < tl {
            ti += 1;
        }
        b0 = bi;
        t0 = ti;
    }
    if &bb[b0..bi] == b".." {
        return Err(format!("Rel: can't make {targpath} relative to {basepath}"));
    }
    if b0 != bl {
        let seps = base[b0..bl].matches('/').count();
        let mut out = String::from("..");
        for _ in 0..seps {
            out.push_str("/..");
        }
        if t0 != tl {
            out.push('/');
            out.push_str(&targ[t0..]);
        }
        return Ok(out);
    }
    Ok(targ[t0..].to_string())
}

/// Go's `filepath.ToSlash`. A no-op on Unix; kept because the original calls it
/// on the rc path before handing it to bash.
pub fn to_slash(path: &str) -> String {
    path.to_string()
}

/// Go's `filepath.EvalSymlinks`.
pub fn eval_symlinks(path: &str) -> io::Result<String> {
    let resolved = std::fs::canonicalize(path)?;
    Ok(resolved.to_string_lossy().into_owned())
}

/// Go's `filepath.Walk` — pre-order, lexical within a directory, root first.
///
/// The visitor sees the same paths Go's would, built by string concatenation
/// from `root` rather than re-derived from the directory entries.
pub fn walk<F>(root: &str, visit: &mut F) -> io::Result<()>
where
    F: FnMut(&str, &std::fs::Metadata) -> io::Result<()>,
{
    let meta = std::fs::symlink_metadata(root)?;
    walk_inner(root, &meta, visit)
}

fn walk_inner<F>(path: &str, meta: &std::fs::Metadata, visit: &mut F) -> io::Result<()>
where
    F: FnMut(&str, &std::fs::Metadata) -> io::Result<()>,
{
    visit(path, meta)?;
    if !meta.is_dir() {
        return Ok(());
    }
    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(path)? {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    for name in names {
        let child = join(&[path, &name]);
        let child_meta = std::fs::symlink_metadata(&child)?;
        walk_inner(&child, &child_meta, visit)?;
    }
    Ok(())
}

/// The current working directory as a string, or an error the way Go words it.
pub fn getwd() -> io::Result<String> {
    Ok(std::env::current_dir()?.to_string_lossy().into_owned())
}

/// True when `path` names an existing entry, following symlinks.
pub fn exists(path: &str) -> bool {
    Path::new(path).exists()
}
