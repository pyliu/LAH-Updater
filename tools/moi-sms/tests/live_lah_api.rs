use moi_sms::lah_client::{LahApiClient, QueueEnqueueRequest};

#[test]
#[ignore = "需要本機 Apache LAH-API 服務啟動才執行"]
fn test_live_lah_api_endpoints_roundtrip() {
    let base_url = std::env::var("LAH_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1/api/sms_v1_api.php".to_string());
    let client = LahApiClient::new(&base_url).expect("valid LAH-API URL");

    let meta = client.get_metadata().expect("GET /metadata");
    assert!(!meta.site_id.is_empty(), "site_id should not be empty");
    assert!(!meta.office_name.is_empty(), "office_name should not be empty");

    let sync = client.sync_cases(None).expect("POST /cases/sync");
    assert_eq!(sync.target_date.len(), 7, "target_date should be ROC YYYMMDD");

    let enqueue = client
        .enqueue_queue(&QueueEnqueueRequest::default())
        .expect("POST /queue/enqueue");
    assert_eq!(enqueue.moved_to_wait_count, enqueue.deleted_from_main_count);

    let pending = client.get_pending_queue().expect("GET /queue/pending");
    assert_eq!(pending.site_id, meta.site_id);

    for table in ["main", "wait", "log"] {
        let recs = client.get_records(table, 50).expect("GET /records");
        assert_eq!(recs.table, table);
        assert_eq!(recs.count, recs.records.len());
    }
}
