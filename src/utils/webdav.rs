use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use std::io::Cursor;
use xml::writer::XmlEvent;
use xml::EmitterConfig;

pub enum S3ObjectOutput {
    GetObject(GetObjectOutput),
    ListObjects(ListObjectsV2Output),
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

fn propfind_single(bucket: &str, key: &str, _objects: GetObjectOutput) -> String {
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
    writer.write(XmlEvent::characters(key)).unwrap();
    writer.write(XmlEvent::end_element()).unwrap(); // displayname

    writer
        .write(XmlEvent::start_element("getcontentlength"))
        .unwrap();
    writer.write(XmlEvent::characters("0")).unwrap();
    writer.write(XmlEvent::end_element()).unwrap(); // getcontentlength

    writer
        .write(XmlEvent::start_element("getlastmodified"))
        .unwrap();
    writer
        .write(XmlEvent::characters("2021-01-01T00:00:00Z"))
        .unwrap();
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

    // File responses
    for object in objects.contents() {
        let key = object.key().unwrap_or("");
        let size = object.size();
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
        writer.write(XmlEvent::characters(&key)).unwrap();
        writer.write(XmlEvent::end_element()).unwrap(); // displayname

        writer
            .write(XmlEvent::start_element("getcontentlength"))
            .unwrap();
        writer
            .write(XmlEvent::characters(&size.unwrap().to_string()))
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
