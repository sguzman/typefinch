//! Explicit, optional public GitHub card publishing. No stored tokens,
//! background synchronization or arbitrary user-selected repository paths.
//! The GitHub CLI owns authentication; this code never reads credentials.
use base64::{engine::general_purpose::STANDARD,Engine as _};
use serde_json::{json,Value};
use std::{io::{Read,Write},process::{Command,Stdio},thread};

const CARD_PATH:&str="typefinch-profile.svg";
const MAX_REMOTE_JSON:usize=2_500_000;
const MAX_REMOTE_STDERR:usize=64*1024;

/// Enforce response bounds *while* consuming a pipe, not after an
/// unbounded wait_with_output allocation. The extra byte distinguishes
/// an exact-size response from an oversized or malicious API reply.
fn read_bounded<R:Read>(reader:R,limit:usize)->Result<Vec<u8>,()> {
    let mut result=Vec::new();
    reader.take((limit+1) as u64).read_to_end(&mut result).map_err(|_|())?;
    if result.len()>limit {return Err(());}
    Ok(result)
}

#[derive(Debug,PartialEq,Eq)]
pub enum ApiFailure {NotFound,Unavailable}
/// Narrow API seam: pure tests never connect to GitHub or invoke gh.
pub trait Api {
    fn get(&self,endpoint:&str)->Result<Value,ApiFailure>;
    fn put(&self,endpoint:&str,body:&Value)->Result<Value,ApiFailure>;
}
pub struct Gh;
impl Gh {
    fn call(&self,endpoint:&str,body:Option<&Value>)->Result<Value,ApiFailure> {
        // Linux coreutils imposes a process-wide deadline even if the CLI
        // authentication helper or network hangs. Never invoke a shell.
        let mut cmd=Command::new("timeout");
        cmd.args(["-k","2s","25s","gh","api","--hostname","github.com",endpoint]);
        if body.is_some() {cmd.args(["--method","PUT","--input","-"]);}
        cmd.env("GH_PROMPT_DISABLED","1")
            .stdin(if body.is_some(){Stdio::piped()}else{Stdio::null()})
            .stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child=cmd.spawn().map_err(|_|ApiFailure::Unavailable)?;
        let stdout=child.stdout.take().ok_or(ApiFailure::Unavailable)?;
        let stderr=child.stderr.take().ok_or(ApiFailure::Unavailable)?;
        let out=thread::spawn(move||read_bounded(stdout,MAX_REMOTE_JSON));
        let err=thread::spawn(move||read_bounded(stderr,MAX_REMOTE_STDERR));
        if let Some(data)=body {
            let bytes=serde_json::to_vec(data).map_err(|_|ApiFailure::Unavailable)?;
            let Some(mut input)=child.stdin.take() else{return Err(ApiFailure::Unavailable)};
            if input.write_all(&bytes).is_err(){
                let _=child.kill();
                let _=child.wait();
                return Err(ApiFailure::Unavailable);
            }
            drop(input);
        }
        let status=child.wait().map_err(|_|ApiFailure::Unavailable)?;
        let stdout=out.join().map_err(|_|ApiFailure::Unavailable)?
            .map_err(|_|ApiFailure::Unavailable)?;
        let stderr=err.join().map_err(|_|ApiFailure::Unavailable)?
            .map_err(|_|ApiFailure::Unavailable)?;
        if !status.success() {
            let error=String::from_utf8_lossy(&stderr);
            return Err(if error.contains("HTTP 404") {
                ApiFailure::NotFound
            } else {ApiFailure::Unavailable});
        }
        serde_json::from_slice(&stdout).map_err(|_|ApiFailure::Unavailable)
    }
}
impl Api for Gh {
    fn get(&self,endpoint:&str)->Result<Value,ApiFailure> {self.call(endpoint,None)}
    fn put(&self,endpoint:&str,body:&Value)->Result<Value,ApiFailure> {
        self.call(endpoint,Some(body))
    }
}
/// Only a conventional GitHub.com owner/repository pair is allowed. This
/// rejects hosts, paths, querystrings and shell control characters.
pub fn repository(input:&str)->Result<String,String> {
    let value=input.trim();
    let Some((owner,repo))=value.split_once('/') else{
        return Err("Enter a GitHub.com repository as owner/repository.".into());
    };
    let owner_ok=!owner.is_empty() && owner.len()<=39
        && !owner.starts_with('-') && !owner.ends_with('-')
        && owner.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-');
    let repo_ok=!repo.is_empty() && repo.len()<=100 && repo!="." && repo!=".."
        && repo.bytes().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'.'|b'_'|b'-'));
    if !owner_ok || !repo_ok {
        return Err("Repository must be a GitHub.com owner/repository without paths or punctuation.".into());
    }
    Ok(format!("{owner}/{repo}"))
}
// The exporter emits a fixed vocabulary of SVG text and geometry only.
// Reject scripts, remote images, CSS, animation, embedded documents and
// event handlers even if a caller supplies forged Typefinch aria-labels.
// No XML parser/new runtime dependencies or network fetching are necessary.
fn safe_attribute(name:&str,value:&str)->bool {
    match name {
        "xmlns"=>value=="http://www.w3.org/2000/svg",
        "role"=>value=="img",
        "aria-label"=>matches!(value,"Typefinch profile"|"Typefinch compact profile"),
        "font-family"=>matches!(value,"sans-serif"|"monospace"),
        "font-weight"=>value=="700",
        "fill"|"stroke"=>value=="none"
            || (value.len()==7 && value.starts_with('#')
                && value[1..].bytes().all(|b|b.is_ascii_hexdigit())),
        "x"|"y"|"cx"|"cy"|"r"|"rx"|"ry"|"width"|"height"|
        "stroke-width"|"font-size"=>!value.is_empty()
            && value.bytes().all(|b|b.is_ascii_digit() || b==b'.' || b==b'-'),
        "viewBox"=>!value.is_empty()
            && value.bytes().all(|b|b.is_ascii_digit() || b==b'.' || b==b'-' || b==b' '),
        "d"=>!value.is_empty()
            && value.bytes().all(|b|b.is_ascii_alphabetic()
                || b.is_ascii_digit() || matches!(b,b' '|b'.'|b'-'|b',')),
        _=>false,
    }
}
fn static_typefinch_svg(svg:&str)->bool {
    if svg.len()>64*1024 || !svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\"")
        || !svg.ends_with("</svg>") {return false;}
    let mut pieces=svg.split('<');
    if pieces.next()!=Some(""){return false;}
    let (mut opened,mut closed,mut in_text,mut saw_text)=(false,false,false,false);
    for fragment in pieces {
        let Some((raw,text))=fragment.split_once('>') else{return false;};
        if closed {return false;}
        if raw=="/text" {
            if !in_text {return false;}
            in_text=false;
            continue;
        }
        if raw=="/svg" {
            if !opened || in_text || !text.is_empty() {return false;}
            closed=true;
            continue;
        }
        if in_text {return false;}
        let self_closing=raw.ends_with('/');
        let content=raw.strip_suffix('/').unwrap_or(raw).trim_end();
        let end=content.find(char::is_whitespace).unwrap_or(content.len());
        let (tag,remaining)=content.split_at(end);
        if !matches!(tag,"svg"|"rect"|"text"|"ellipse"|"circle"|"path"){return false;}
        if tag=="svg" {
            if opened || self_closing {return false;}
            opened=true;
        } else {
            if !opened || self_closing==(tag=="text"){return false;}
            if tag=="text" {in_text=true;saw_text=true;}
        }
        let mut attrs=remaining.trim();
        let mut unique=std::collections::HashSet::new();
        while !attrs.is_empty(){
            let Some(equal)=attrs.find('=') else{return false;};
            let key=attrs[..equal].trim_end();
            if key.is_empty() || key.chars().any(char::is_whitespace)
                || !unique.insert(key) {return false;}
            let after=attrs[equal+1..].trim_start();
            let Some(quoted)=after.strip_prefix('"') else{return false;};
            let Some(end)=quoted.find('"') else{return false;};
            if !safe_attribute(key,&quoted[..end]) {return false;}
            attrs=quoted[end+1..].trim_start();
        }
        if tag=="svg" && (!content.contains("aria-label=") || !content.contains("viewBox=")){
            return false;
        }
        // The fragment's text belongs to the preceding opening tag. XML
        // special characters in user-supplied profile labels are escaped
        // by the existing exporter; raw '<' becomes an unknown tag above.
        if tag!="text" && !text.trim().is_empty() {return false;}
    }
    opened && closed && !in_text && saw_text
}
fn owned_card(data:&Value)->Result<(String,Vec<u8>),String> {
    if data.get("type").and_then(Value::as_str)!=Some("file") {
        return Err("Existing GitHub path is not an ordinary file; it will not be overwritten.".into());
    }
    let sha=data.get("sha").and_then(Value::as_str).filter(|sha|
        sha.len()==40 && sha.bytes().all(|b|b.is_ascii_hexdigit()))
        .ok_or("Cannot validate existing remote file identity; upload refused.")?;
    if data.get("encoding").and_then(Value::as_str)!=Some("base64") {
        return Err("Cannot inspect existing file content; upload refused.".into());
    }
    let encoded=data.get("content").and_then(Value::as_str)
        .ok_or("Existing file content not readable; upload refused.")?
        .chars().filter(|ch|!ch.is_ascii_whitespace()).collect::<String>();
    if encoded.len()>1_500_000 {return Err("Remote content too large; upload refused.".into());}
    let bytes=STANDARD.decode(encoded).map_err(|_|"Cannot decode remote file; upload refused.")?;
    let current=std::str::from_utf8(&bytes)
        .map_err(|_|"Existing file is not UTF-8 SVG; upload refused.")?;
    if !static_typefinch_svg(current) {
        return Err("Existing file does not identify as a safe Typefinch card; refusing overwrite.".into());
    }
    Ok((sha.to_owned(),bytes))
}
/// Every call is authorized by a fresh user-facing consent checkbox and
/// concrete click, and only the fixed card path can be updated. The SVG
/// string must already be produced by Typefinch's safe XML exporter.
pub fn publish_via<A:Api>(api:&A,input:&str,svg:&str,consent:bool)->Result<String,String> {
    if !consent {return Err("Public GitHub publishing requires explicit consent.".into());}
    let repo=repository(input)?;
    if !static_typefinch_svg(svg) {
        return Err("Only a Typefinch-generated static SVG can be uploaded.".into());
    }
    let repo_endpoint=format!("repos/{repo}");
    let info=api.get(&repo_endpoint).map_err(|_|
        "Cannot verify the target GitHub repository or credentials; nothing was uploaded.".to_owned())?;
    if info.get("permissions").and_then(|p|p.get("push")).and_then(Value::as_bool)!=Some(true) {
        return Err("Current GitHub CLI credentials cannot confirm write permission; nothing was uploaded.".into());
    }
    // "Public card" must not silently target a private or organization-
    // internal repository. Both GitHub's visibility and private fields
    // must explicitly identify a public repository before any file lookup,
    // and especially before attempting a write.
    if info.get("visibility").and_then(Value::as_str)!=Some("public")
        || info.get("private").and_then(Value::as_bool)!=Some(false) {
        return Err("The selected GitHub repository is not verified public; nothing was uploaded.".into());
    }
    // GitHub transparently redirects renamed repositories. Never send a
    // public card to a different canonical destination while reporting the
    // original typed owner/repository as the confirmed publication target.
    if !info.get("full_name").and_then(Value::as_str)
        .is_some_and(|name|name.eq_ignore_ascii_case(&repo)) {
        return Err("GitHub resolved a different repository identity; nothing was uploaded.".into());
    }
    let card_endpoint=format!("{repo_endpoint}/contents/{CARD_PATH}");
    let old=match api.get(&card_endpoint) {
        Ok(file)=>Some(owned_card(&file)?.0),
        Err(ApiFailure::NotFound)=>None,
        Err(ApiFailure::Unavailable)=>return Err(
            "Cannot verify the existing card (network or authentication failure); nothing was uploaded.".into()),
    };
    let mut body=json!({
        "message":"Update Typefinch public profile card",
        "content":STANDARD.encode(svg.as_bytes())
    });
    if let Some(sha)=old {body["sha"]=Value::String(sha);}
    let receipt=api.put(&card_endpoint,&body).map_err(|_|
        "GitHub declined or could not complete the upload; check the repository manually.".to_owned())?;
    if receipt.get("content").and_then(|c|c.get("path")).and_then(Value::as_str)!=Some(CARD_PATH){
        return Err("GitHub response did not confirm the card path; inspect the repository.".into());
    }
    // A successful PUT response with a filename is not proof that the
    // intended bytes are actually readable in the public repository. Do
    // not display a publication-success link until GitHub returns exactly
    // our newly submitted SVG from the fixed Contents API path.
    // A read-back failure may follow a real write (e.g., network outage
    // or API cache delay), so never claim that no upload occurred here.
    let stored=api.get(&card_endpoint).map_err(|_|
        "GitHub accepted an upload, but read-back could not verify it; inspect the repository.".to_owned())?;
    let (_,bytes)=owned_card(&stored).map_err(|_|
        "GitHub read-back did not verify a Typefinch card; inspect the repository.".to_owned())?;
    if bytes!=svg.as_bytes() {
        return Err("GitHub read-back did not match the uploaded card; inspect the repository.".into());
    }
    Ok(format!("https://github.com/{repo}"))
}
pub fn publish(input:&str,svg:&str,consent:bool)->Result<String,String> {
    publish_via(&Gh,input,svg,consent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell,collections::VecDeque};
    struct Mock {
        replies:RefCell<VecDeque<Result<Value,ApiFailure>>>,
        calls:RefCell<Vec<(String,Option<Value>)>>,
    }
    impl Mock {
        fn new(responses:Vec<Result<Value,ApiFailure>>)->Self {
            Self{replies:RefCell::new(responses.into()),calls:RefCell::new(Vec::new())}
        }
        fn next(&self,path:&str,body:Option<&Value>)->Result<Value,ApiFailure>{
            self.calls.borrow_mut().push((path.to_owned(),body.cloned()));
            self.replies.borrow_mut().pop_front().expect("unexpected API call")
        }
    }
    impl Api for Mock {
        fn get(&self,path:&str)->Result<Value,ApiFailure>{self.next(path,None)}
        fn put(&self,path:&str,body:&Value)->Result<Value,ApiFailure>{
            self.next(path,Some(body))
        }
    }
    const DEMO:&str="<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"550\" height=\"145\" viewBox=\"0 0 550 145\" role=\"img\" aria-label=\"Typefinch compact profile\"><text x=\"23\" y=\"31\" fill=\"#FFFFFF\" font-size=\"20\" font-family=\"sans-serif\" font-weight=\"700\">Typefinch</text></svg>";
    fn repo_info()->Value{json!({"permissions":{"push":true},"private":false,"visibility":"public","full_name":"alice/cards"})}
    fn uploaded()->Value{json!({"content":{"path":"typefinch-profile.svg"}})}
    fn verified_card()->Value{json!({"type":"file","sha":"a".repeat(40),
        "encoding":"base64","content":STANDARD.encode(DEMO)})}
    #[test] fn api_subprocess_output_is_bounded_before_allocation(){
        use std::io::Cursor;
        assert_eq!(read_bounded(Cursor::new(b"1234"),4).unwrap(),b"1234");
        assert!(read_bounded(Cursor::new(b"12345"),4).is_err());
        let huge=vec![b'x';MAX_REMOTE_STDERR+1];
        assert!(read_bounded(Cursor::new(huge),MAX_REMOTE_STDERR).is_err());
        let example=serde_json::to_vec(&repo_info()).unwrap();
        assert_eq!(read_bounded(Cursor::new(&example),MAX_REMOTE_JSON).unwrap(),example);
    }
    #[test] fn rejects_nonconforming_target_and_missing_consent_without_api_access(){
        for target in ["owner/repo/README.md","https://github.com/owner/repo",
            "owner/repo?x=1","evil;rm/repo","owner/..","-/repo","owner/repo and more"]{
            assert!(repository(target).is_err(),"{target}");
        }
        assert_eq!(repository("  alice/typefinch-card ").unwrap(),"alice/typefinch-card");
        let mock=Mock::new(vec![]);
        assert!(publish_via(&mock,"alice/cards",DEMO,false).unwrap_err().contains("consent"));
        assert!(mock.calls.borrow().is_empty());
    }
    #[test] fn all_real_exported_card_styles_remain_safe_to_publish(){
        use crate::{export,preferences::{CardTheme,CardFont,CardTemplate,Profile}};
        let profile=Profile {name:"<script> & שָׁלוֹם 🐦".into(),..Profile::default()};
        for theme in CardTheme::ALL {
            for font in CardFont::ALL {
                for template in CardTemplate::ALL {
                    let svg=export::card_with_template(&profile,
                        "de(neo) & <layout>",theme,font,template);
                    assert!(static_typefinch_svg(&svg),
                        "Actual locally generated SVG must satisfy public-card safety rules");
                    let remote=json!({"type":"file","sha":"a".repeat(40),
                        "encoding":"base64","content":STANDARD.encode(svg.as_bytes())});
                    assert_eq!(owned_card(&remote).unwrap().1,svg.as_bytes(),
                        "Valid preexisting Typefinch card must remain updateable");
                }
            }
        }
    }
    #[test] fn uploaded_svg_markup_is_static_allowlisted_and_user_text_is_escaped(){
        assert!(static_typefinch_svg(DEMO));
        let safe=DEMO.replace("Typefinch</text>","&lt;script&gt; Typefinch &amp; 你好</text>");
        assert!(static_typefinch_svg(&safe));
        for fragment in [
            "<script>alert(1)</script>",
            "<foreignObject><body>unsafe</body></foreignObject>",
            "<image href=\"https://example.test/tracker\"/>",
            "<rect onload=\"alert(1)\"/>",
            "<rect style=\"fill:url(https://example.test)\"/>",
            "<animate attributeName=\"fill\"/>",
            "<!-- hidden content -->",
        ] {
            let payload=DEMO.replace("</svg>",&format!("{fragment}</svg>"));
            assert!(!static_typefinch_svg(&payload), "unapproved SVG markup");
            let mock=Mock::new(vec![]);
            assert!(publish_via(&mock,"alice/cards",&payload,true).is_err());
            assert!(mock.calls.borrow().is_empty(),"unsafe card must not access GitHub");
        }
        assert!(!static_typefinch_svg(&"A".repeat(65*1024)));
    }
    #[test] fn renamed_or_unidentified_repository_never_receives_public_card(){
        for full_name in [None,Some("alice/redirected"),Some("someone/else")] {
            let mut info=repo_info();
            if let Some(name)=full_name { info["full_name"]=json!(name); }
            else { info.as_object_mut().unwrap().remove("full_name"); }
            let mock=Mock::new(vec![Ok(info)]);
            assert!(publish_via(&mock,"alice/cards",DEMO,true).unwrap_err()
                .contains("different repository identity"));
            assert_eq!(mock.calls.borrow().len(),1);
            assert!(mock.calls.borrow().iter().all(|(_,body)|body.is_none()));
        }
        let mut canonical=repo_info();
        canonical["full_name"]=json!("Alice/CARDS");
        let mock=Mock::new(vec![Ok(canonical),
            Err(ApiFailure::NotFound),Ok(uploaded()),Ok(verified_card())]);
        assert!(publish_via(&mock,"alice/cards",DEMO,true).is_ok(),
            "GitHub repository name comparison must remain case-insensitive");
    }
    #[test] fn private_internal_or_unverified_repositories_never_receive_public_card(){
        for info in [
            json!({"permissions":{"push":true},"private":true,"visibility":"private"}),
            json!({"permissions":{"push":true},"private":false,"visibility":"internal"}),
            json!({"permissions":{"push":true},"private":false}),
            json!({"permissions":{"push":true},"visibility":"public"}),
            json!({"permissions":{"push":true},"private":false,"visibility":"private"}),
        ]{
            let mock=Mock::new(vec![Ok(info)]);
            assert!(publish_via(&mock,"alice/private-cards",DEMO,true).unwrap_err()
                .contains("not verified public"));
            let calls=mock.calls.borrow();
            assert_eq!(calls.len(),1,"nonpublic card must not be looked up or written");
            assert_eq!(calls[0].0,"repos/alice/private-cards");
            assert!(calls[0].1.is_none());
        }
    }
    #[test] fn publishing_new_card_never_handles_tokens_or_other_repository_paths(){
        let mock=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::NotFound),Ok(uploaded()),Ok(verified_card())]);
        assert_eq!(publish_via(&mock,"alice/cards",DEMO,true).unwrap(),
            "https://github.com/alice/cards");
        let c=mock.calls.borrow();
        assert_eq!(c.len(),4);
        assert_eq!(c[0].0,"repos/alice/cards");
        assert_eq!(c[1].0,"repos/alice/cards/contents/typefinch-profile.svg");
        assert_eq!(c[2].0,c[1].0);
        assert_eq!(c[3].0,c[1].0);
        assert!(c[3].1.is_none());
        let body=c[2].1.as_ref().unwrap();
        assert_eq!(body.get("sha"),None);
        assert_eq!(STANDARD.decode(body["content"].as_str().unwrap()).unwrap(),DEMO.as_bytes());
        assert!(!body.to_string().contains("token"));
    }
    #[test] fn existing_non_typefinch_files_are_never_replaced(){
        let foreign=json!({"type":"file","sha":"a".repeat(40),
            "encoding":"base64","content":STANDARD.encode("<svg>not ours</svg>")});
        let mock=Mock::new(vec![Ok(repo_info()),Ok(foreign)]);
        assert!(publish_via(&mock,"alice/cards",DEMO,true).unwrap_err().contains("refusing overwrite"));
        assert_eq!(mock.calls.borrow().len(),2);
    }
    #[test] fn updates_owned_card_only_when_sha_is_verified(){
        let existing=json!({"type":"file","sha":"a".repeat(40),
            "encoding":"base64","content":STANDARD.encode(DEMO)});
        let mock=Mock::new(vec![Ok(repo_info()),Ok(existing),Ok(uploaded()),Ok(verified_card())]);
        publish_via(&mock,"alice/cards",DEMO,true).unwrap();
        assert_eq!(mock.calls.borrow()[2].1.as_ref().unwrap()["sha"],"a".repeat(40));
    }
    #[test] fn upload_receipt_is_not_publication_success_without_exact_readback(){
        let no_readback=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::NotFound),
            Ok(uploaded()),Err(ApiFailure::Unavailable)]);
        assert!(publish_via(&no_readback,"alice/cards",DEMO,true).unwrap_err()
            .contains("read-back could not verify"));
        assert_eq!(no_readback.calls.borrow().len(),4);

        let stale=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::NotFound),
            Ok(uploaded()),Ok(json!({"type":"file","sha":"b".repeat(40),
                "encoding":"base64","content":STANDARD.encode(
                    DEMO.replace(">Typefinch</text>",">Changed</text>"))}))]);
        assert!(publish_via(&stale,"alice/cards",DEMO,true).unwrap_err()
            .contains("did not match"));
        assert_eq!(stale.calls.borrow().len(),4);

        let wrong_kind=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::NotFound),
            Ok(uploaded()),Ok(json!({"type":"dir","sha":"a".repeat(40),
                "encoding":"base64","content":STANDARD.encode(DEMO)}))]);
        assert!(publish_via(&wrong_kind,"alice/cards",DEMO,true).unwrap_err()
            .contains("did not verify"));
        assert_eq!(wrong_kind.calls.borrow().len(),4);
    }
    #[test] fn rejects_foreign_or_unverifiable_remote_card_without_put(){
        for invalid in [
            json!({"type":"dir","sha":"a".repeat(40),"encoding":"base64","content":STANDARD.encode(DEMO)}),
            json!({"type":"file","sha":"wrong","encoding":"base64","content":STANDARD.encode(DEMO)}),
            json!({"type":"file","sha":"a".repeat(40),"encoding":"none","content":STANDARD.encode(DEMO)}),
            json!({"type":"file","sha":"a".repeat(40),"encoding":"base64","content":"%%%"}),
            json!({"type":"file","sha":"a".repeat(40),"encoding":"base64","content":STANDARD.encode([0xff,0xfe])}),
            json!({"type":"file","sha":"a".repeat(40),"encoding":"base64","content":STANDARD.encode("<svg xmlns=\\\"http://www.w3.org/2000/svg\\\">foreign</svg>")}),
        ] {
            let mock=Mock::new(vec![Ok(repo_info()),Ok(invalid)]);
            assert!(publish_via(&mock,"alice/cards",DEMO,true).is_err());
            let calls=mock.calls.borrow();
            assert_eq!(calls.len(),2,"invalid card must be rejected before PUT");
            assert!(calls.iter().all(|(_,body)|body.is_none()));
        }
    }
    #[test] fn missing_ownership_and_upload_failure_are_not_success(){
        let fake=Mock::new(vec![Ok(json!({"permissions":{}}))]);
        assert!(publish_via(&fake,"alice/cards",DEMO,true).unwrap_err().contains("permission"));
        assert_eq!(fake.calls.borrow().len(),1);
        let failed=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::NotFound),Err(ApiFailure::Unavailable)]);
        assert!(publish_via(&failed,"alice/cards",DEMO,true).unwrap_err().contains("check the repository"));
        assert_eq!(failed.calls.borrow().len(),3);
        let wrong_receipt=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::NotFound),
            Ok(json!({"content":{"path":"README.md"}}))]);
        assert!(publish_via(&wrong_receipt,"alice/cards",DEMO,true).unwrap_err().contains("did not confirm"));
        let bad_svg=Mock::new(vec![]);
        assert!(publish_via(&bad_svg,"alice/cards","<svg><script/></svg>",true).is_err());
        assert!(bad_svg.calls.borrow().is_empty());
    }
    #[test] fn offline_and_missing_permissions_fail_closed(){
        let missing_permission=Mock::new(vec![Ok(json!({"permissions":{"push":false}}))]);
        assert!(publish_via(&missing_permission,"alice/cards",DEMO,true).is_err());
        assert_eq!(missing_permission.calls.borrow().len(),1);
        let outage=Mock::new(vec![Ok(repo_info()),Err(ApiFailure::Unavailable)]);
        assert!(publish_via(&outage,"alice/cards",DEMO,true).unwrap_err().contains("nothing was uploaded"));
        assert_eq!(outage.calls.borrow().len(),2);
        let unauth=Mock::new(vec![Err(ApiFailure::Unavailable)]);
        assert!(publish_via(&unauth,"alice/cards",DEMO,true).is_err());
        assert_eq!(unauth.calls.borrow().len(),1);
    }
}
