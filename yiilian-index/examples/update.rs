
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::TermQuery;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::IndexReader;

fn extract_doc_given_isbn(
    reader: &IndexReader,
    isbn_term: &Term,
) -> tantivy::Result<Option<Document>> {
    let searcher = reader.searcher();

    let term_query = TermQuery::new(isbn_term.clone(), IndexRecordOption::Basic);
    let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
    if let Some((_score, doc_address)) = top_docs.first() {
        let doc = searcher.doc(*doc_address)?;
        Ok(Some(doc))
    } else {
        Ok(None)
    }
}

fn main() -> tantivy::Result<()> {
    let mut schema_builder = Schema::builder();
    let isbn = schema_builder.add_text_field("isbn", STRING | STORED);
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let schema = schema_builder.build();

    let index = Index::create_in_ram(schema.clone());

    let mut index_writer = index.writer(50_000_000)?;

    let mut old_man_doc = Document::default();
    old_man_doc.add_text(title, "The Old Man and the Sea");
    index_writer.add_document(doc!(
        isbn => "978-0099908401",
        title => "The old Man and the see"
    )).unwrap();
    index_writer.add_document(doc!(
        isbn => "978-0140177398",
        title => "Of Mice and Men",
    )).unwrap();
    index_writer.add_document(doc!(
       title => "Frankentein", //< Oops there is a typo here.
       isbn => "978-9176370711",
    )).unwrap();
    index_writer.commit()?;
    let reader = index.reader()?;

    let frankenstein_isbn = Term::from_field_text(isbn, "978-9176370711");

    let frankenstein_doc_misspelled = extract_doc_given_isbn(&reader, &frankenstein_isbn)?.unwrap();
    assert_eq!(
        schema.to_json(&frankenstein_doc_misspelled),
        r#"{"isbn":["978-9176370711"],"title":["Frankentein"]}"#,
    );

    index_writer.delete_term(frankenstein_isbn.clone());
    index_writer.add_document(doc!(
        title => "Frankenstein",
        isbn => "978-9176370711",
     )).unwrap();
     index_writer.commit()?;

    //  reader.reload()?;
    let reader = index.reader()?;

     let frankenstein_new_doc = extract_doc_given_isbn(&reader, &frankenstein_isbn)?.unwrap();
    assert_eq!(
        schema.to_json(&frankenstein_new_doc),
        r#"{"isbn":["978-9176370711"],"title":["Frankenstein"]}"#,
    );

    Ok(())
}