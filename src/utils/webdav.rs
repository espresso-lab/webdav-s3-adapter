use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use std::io::Cursor;
use std::path::Path;
use xml::writer::XmlEvent;
use xml::EmitterConfig;

pub enum S3ObjectOutput {
    GetObject(GetObjectOutput),
    ListObjects(ListObjectsV2Output),
}

fn get_filename_from_path(path: &str) -> Option<&str> {
    let path = Path::new(path);
    path.file_name()?.to_str()
}

pub fn generate_webdav_propfind_response(
    bucket: &str,
    key: &str,
    objects: S3ObjectOutput,
) -> String {
    match objects {
        S3ObjectOutput::GetObject(get_object_output) => {
            propfind_single(bucket, key, get_object_output)
        }
        S3ObjectOutput::ListObjects(list_objects_output) => {
            propfind_multiple(bucket, key, list_objects_output)
        }
    }
}

fn propfind_single(bucket: &str, key: &str, objects: GetObjectOutput) -> String {
    let file_name = get_filename_from_path(key).unwrap();
    let size = objects.content_length().unwrap_or(0);
    let last_modified = objects.last_modified().unwrap().to_string();
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut buffer);

    writer
        .write(XmlEvent::start_element("multistatus").default_ns("DAV:"))
        .unwrap();

    writer.write(XmlEvent::start_element("response")).unwrap();
    writer.write(XmlEvent::start_element("href")).unwrap();
    writer
        .write(XmlEvent::characters(&format!("/{}/{}", bucket, key)))
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

    writer.write(XmlEvent::end_element()).unwrap(); // multistatus

    String::from_utf8(buffer.into_inner()).unwrap()
}

fn propfind_multiple(bucket: &str, key: &str, objects: ListObjectsV2Output) -> String {
    let mut buffer = Cursor::new(Vec::new());
    let mut writer = EmitterConfig::new()
        .perform_indent(true)
        .create_writer(&mut buffer);

    writer
        .write(XmlEvent::start_element("multistatus").default_ns("DAV:"))
        .unwrap();

    // Folder response
    writer.write(XmlEvent::start_element("response")).unwrap();
    writer.write(XmlEvent::start_element("href")).unwrap();
    writer
        .write(XmlEvent::characters(&format!("/{}/{}", bucket, key)))
        .unwrap();
    writer.write(XmlEvent::end_element()).unwrap(); // href

    writer.write(XmlEvent::start_element("propstat")).unwrap();
    writer.write(XmlEvent::start_element("prop")).unwrap();

    writer
        .write(XmlEvent::start_element("displayname"))
        .unwrap();
    writer.write(XmlEvent::characters(key)).unwrap();
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

    for prefix in objects.common_prefixes() {
        let folder_name = get_filename_from_path(prefix.prefix().unwrap()).unwrap();

        writer.write(XmlEvent::start_element("response")).unwrap();
        writer.write(XmlEvent::start_element("href")).unwrap();
        writer
            .write(XmlEvent::characters(&format!(
                "/{}/{}",
                bucket, folder_name
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
            .write(XmlEvent::characters(&format!("/{}/{}", bucket, key)))
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
