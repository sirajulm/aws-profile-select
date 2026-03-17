1. Create a tag

You can do it locally:

git tag v0.1.2
git push origin v0.1.2

Or directly in GitHub UI (Releases → “Draft a new release” → it creates the tag for you).

2. Create a GitHub Release

In GitHub:

Go to Releases

Click “Draft a new release”

Choose your tag (v0.1.2)

Add title/notes

3. Attach compiled binaries

This is the important part for mise/ubi

You need to upload files like:

aws-profile-select-x86_64-unknown-linux-gnu
aws-profile-select-aarch64-apple-darwin
aws-profile-select-x86_64-apple-darwin

👉 These are just compiled outputs from:

cargo build --release

(or cross-compiled)

⚙️ Automating this (highly recommended)

Doing this manually gets old fast. Most people use GitHub Actions to:

build for multiple platforms

create release

upload binaries automatically

Typical tools:

cross (for cross-compilation)

cargo-dist (very nice all-in-one)

release-plz or cargo-release

If you want the smoothest path → cargo-dist is the current gold standard.

🚀 Then mise usage becomes clean

Once releases exist:

mise use -g ubi:OWNER/aws-profile-select

And updating:

mise upgrade aws-profile-select

👉 No compiling, just downloads a binary like a real package manager.

⚠️ Small but important detail

ubi expects:

consistent naming of binaries

standard target triples in filenames

If naming is weird, installs can fail.

💡 Reality check

Yes, GitHub supports everything natively ✅

The missing piece is just building and attaching binaries

If you want, I can:

generate a ready-to-use GitHub Actions workflow for your repo

or set up cargo-dist so this becomes fully automatic in ~5 minutes