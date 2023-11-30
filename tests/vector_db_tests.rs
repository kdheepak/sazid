#[cfg(test)]
mod vector_db_tests {
  use sazid::app::vector_db::VectorDB;

  #[tokio::test]
  async fn test_connection() {
    let db = VectorDB::new("host=localhost user=tenkai dbname=postgres").await;
    assert!(db.is_ok());
  }

  #[tokio::test]
  async fn test_insert_vector() {
    let db = VectorDB::new("host=localhost user=tenkai dbname=postgres").await.expect("Failed to create VectorDB");
    assert!(db.insert_vector(&[1.0, 2.0, 3.0]).await.is_ok());
  }

  #[tokio::test]
  async fn test_query_vectors() {
    let db = VectorDB::new("host=localhost user=tenkai dbname=postgres").await.expect("Failed to create VectorDB");
    db.insert_vector(&[1.0, 2.0, 3.0]).await.expect("Failed to insert vector");
    let vectors = db.query_vectors(&[1.0, 2.0, 3.0], 5).await.expect("Failed to query vectors");
    assert!(!vectors.is_empty(), "No vectors found");
  }

  #[tokio::test]
  async fn test_enable_extension() {
    let db = VectorDB::new("host=localhost user=tenkai dbname=postgres").await.expect("Failed to create VectorDB");
    assert!(VectorDB::enable_extension(&db.client).await.is_ok());
  }
}
