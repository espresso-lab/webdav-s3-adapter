use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use std::io::Cursor;
use std::path::Path;
use xml::writer::XmlEvent;
use xml::EmitterConfig;

fn get_filename_from_path(path: &str) -> Option<&str> {
    let path = Path::new(path);
    path.file_name()?.to_str()
}

pub fn generate_webdav_propfind_response(bucket: &str, objects: ListObjectsV2Output) -> String {
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut buffer);

    writer
        .write(XmlEvent::start_element("multistatus").default_ns("DAV:"))
        .unwrap();

    // Folder response
    for prefix in objects.common_prefixes() {
        let folder_name = get_filename_from_path(prefix.prefix().unwrap())
            .unwrap()
            .trim_end_matches('/');
        writer.write(XmlEvent::start_element("response")).unwrap();
        writer.write(XmlEvent::start_element("href")).unwrap();
        writer
            .write(XmlEvent::characters(&format!(
                "/{}/{}",
                bucket,
                prefix.prefix().unwrap()
            )))
            .unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // href

        writer.write(XmlEvent::start_element("propstat")).unwrap();
        writer.write(XmlEvent::start_element("prop")).unwrap();

        writer
            .write(XmlEvent::start_element("displayname"))
            .unwrap();
        writer.write(XmlEvent::characters(folder_name)).unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // displayname

        writer
            .write(XmlEvent::start_element("resourcetype"))
            .unwrap();
        writer.write(XmlEvent::start_element("collection")).unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // collection
        writer.write(XmlEvent::end_element()).unwrap(); // resourcetype

        writer.write(XmlEvent::end_element()).unwrap(); // prop
        writer.write(XmlEvent::start_element("status")).unwrap();
        writer
            .write(XmlEvent::characters("HTTP/1.1 200 OK"))
            .unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // status
        writer.write(XmlEvent::end_element()).unwrap(); // propstat

        writer.write(XmlEvent::end_element()).unwrap(); // response
    }

    // File responses
    for object in objects.contents() {
        let file_name = get_filename_from_path(object.key().unwrap()).unwrap();
        let size = object.size().unwrap_or(0);
        let last_modified = object.last_modified().unwrap().to_string();

        writer.write(XmlEvent::start_element("response")).unwrap();
        writer.write(XmlEvent::start_element("href")).unwrap();
        writer
            .write(XmlEvent::characters(&format!(
                "/{}/{}",
                bucket,
                object.key().unwrap()
            )))
            .unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // href

        writer.write(XmlEvent::start_element("propstat")).unwrap();
        writer.write(XmlEvent::start_element("prop")).unwrap();

        writer
            .write(XmlEvent::start_element("displayname"))
            .unwrap();
        writer.write(XmlEvent::characters(file_name)).unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // displayname

        writer
            .write(XmlEvent::start_element("getcontentlength"))
            .unwrap();
        writer
            .write(XmlEvent::characters(&size.to_string()))
            .unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // getcontentlength

        writer
            .write(XmlEvent::start_element("getlastmodified"))
            .unwrap();
        writer.write(XmlEvent::characters(&last_modified)).unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // getlastmodified

        writer
            .write(XmlEvent::start_element("resourcetype"))
            .unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // resourcetype

        writer.write(XmlEvent::end_element()).unwrap(); // prop
        writer.write(XmlEvent::start_element("status")).unwrap();
        writer
            .write(XmlEvent::characters("HTTP/1.1 200 OK"))
            .unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // status
        writer.write(XmlEvent::end_element()).unwrap(); // propstat

        writer.write(XmlEvent::end_element()).unwrap(); // response
    }

    writer.write(XmlEvent::end_element()).unwrap(); // multistatus

    String::from_utf8(buffer.into_inner()).unwrap()
}
