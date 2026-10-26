-- Deterministic, allowlisted Typefinch Linux preview packaging (Lua 5.4).
local version,sha,output=arg[1],arg[2],arg[3] or "dist"
assert(version and version:match("^%d+%.%d+%.%d+$"),"expected X.Y.Z")
assert(sha and sha:match("^[0-9a-f]+$") and #sha==40,"expected full commit SHA")
assert(output~="" and output~="/" and not output:find("//",1,true)
    and output:sub(-1)~="/","unsafe output path")
local function q(x)return "'"..x:gsub("'","'\\''").."'" end
local function run(command)
    local ok,_,code=os.execute(command)
    assert(ok==true and code==0,"command failed: "..command)
end
local function exists(path)
    local ok=os.execute("test -e "..q(path).." || test -L "..q(path))
    return ok==true
end
-- An output symlink or ancestor redirects both the archive and the temporary
-- staging directory. Never follow it, including when a custom output is used.
local ancestor=output:sub(1,1)=="/" and "/" or "."
for segment in output:gmatch("[^/]+") do
    assert(segment~="." and segment~="..","unsafe output component")
    ancestor=(ancestor=="/" and "/"..segment or ancestor.."/"..segment)
    local ok=os.execute("test ! -L "..q(ancestor))
    assert(ok==true,"symlinked package output or ancestor: "..ancestor)
end
local file=assert(io.open("Cargo.toml","rb"))
local toml=assert(file:read("*a"));file:close()
local declared=toml:match('%[package%]%s*.-\nversion%s*=%s*"([^"]+)"')
assert(declared==version,"version mismatch: "..tostring(declared))
local stem="typefinch-"..version.."-linux-x86_64"
local archive=output.."/"..stem..".tar.gz"
local checksum=archive..".sha256"
-- Refuse an archive collision before creating or deleting ANY source, temp,
-- or output file. An old release archive is not disposable scratch data.
assert(not exists(archive) and not exists(checksum),
    "release archive or checksum already exists; refusing overwrite")
run("mkdir -p "..q(output))
-- The former fixed .typefinch-stage path was recursively deleted. Use a
-- uniquely allocated, owned stage instead; stale/unrelated paths survive.
local tmp=assert(io.popen("mktemp -d "..q(output.."/.typefinch-stage-XXXXXX"),"r"))
local stage=tmp:read("*l")
assert(tmp:close() and stage and stage~="","cannot allocate private staging directory")
local root=stage.."/"..stem
run("mkdir -p "..q(root.."/bin").." "..q(root.."/share/doc/typefinch"))
run("install -m 755 target/release/typefinch "..q(root.."/bin/typefinch"))
run("install -m 755 tools/install_release.sh "..q(root.."/install.sh"))
run("install -m 644 README.md "..q(root.."/share/doc/typefinch/README.md"))
run("install -m 644 LICENSE "..q(root.."/share/doc/typefinch/LICENSE"))
-- Retain upstream notices for pinned RTL shaping and JIS physical-key bridges.
for _,vendor in ipairs({"epaint-0.35.0","egui-0.35.0","egui-winit-0.35.0"}) do
    local vendor_docs=root.."/share/doc/typefinch/third-party/"..vendor
    run("mkdir -p "..q(vendor_docs))
    for _,filename in ipairs({"LICENSE-MIT","LICENSE-APACHE","UPSTREAM.md"}) do
        run("install -m 644 "..q("third_party/"..vendor.."/"..filename)..
            " "..q(vendor_docs.."/"..filename))
    end
end
local meta=assert(io.open(root.."/METADATA.txt","wb"))
assert(meta:write("TYPEFINCH_VERSION="..version.."\nGIT_SHA="..sha..
    "\nTARGET=x86_64-unknown-linux-gnu\nCHANNEL=development-preview\n"))
assert(meta:close())
-- noclobber protects against a pre-existing file appearing between the
-- collision preflight and output creation; never silently truncate it.
run("(set -C; tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C "..
    q(stage).." -cf - "..q(stem).." | gzip -n > "..q(archive)..")")
-- Checksum names only the archive basename for standalone verification.
run("(set -C; cd "..q(output).." && sha256sum "..q(stem..".tar.gz")..
    " > "..q(stem..".tar.gz.sha256")..")")
-- Clean up only the uniquely owned stage from THIS successful invocation.
-- Failure leaves that private stage inspectable and never deletes old data.
run("rm -rf -- "..q(stage))
print("Packaged "..archive.." from an explicit, user-data-free allowlist")
