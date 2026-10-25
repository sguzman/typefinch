-- Deterministic, allowlisted Typefinch Linux preview packaging (Lua 5.4).
local version,sha,output=arg[1],arg[2],arg[3] or "dist"
assert(version and version:match("^%d+%.%d+%.%d+$"),"expected X.Y.Z")
assert(sha and sha:match("^[0-9a-f]+$") and #sha==40,"expected full commit SHA")
assert(output~="" and output~="/" and not output:find("..",1,true),"unsafe output path")
local function q(x)return "'"..x:gsub("'","'\\''").."'" end
local function run(command)
    local ok,_,code=os.execute(command)
    assert(ok==true and code==0,"command failed: "..command)
end
local file=assert(io.open("Cargo.toml","rb"))
local toml=assert(file:read("*a"));file:close()
local declared=toml:match('%[package%]%s*.-\nversion%s*=%s*"([^"]+)"')
assert(declared==version,"version mismatch: "..tostring(declared))
local stem="typefinch-"..version.."-linux-x86_64"
local stage=output.."/.typefinch-stage"
local root=stage.."/"..stem
run("mkdir -p "..q(output))
run("rm -rf "..q(stage))
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
local archive=output.."/"..stem..".tar.gz"
run("tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C "..
    q(stage).." -cf - "..q(stem).." | gzip -n > "..q(archive))
-- Checksum names only the archive filename so two downloaded artifacts
-- verify correctly when placed side by side outside the CI dist directory.
run("(cd "..q(output).." && sha256sum "..q(stem..".tar.gz")..
    " > "..q(stem..".tar.gz.sha256")..")")
run("rm -rf "..q(stage))
print("Packaged "..archive.." from an explicit, user-data-free allowlist")
