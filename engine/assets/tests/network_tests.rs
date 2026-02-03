//! Integration tests for asset network transfer.

use engine_assets::{
    AssetId, AssetNetworkClient, AssetNetworkMessage, AssetNetworkServer, TransferPriority,
    TransferStatus,
};

#[test]
fn test_small_asset_transfer() {
    let mut server = AssetNetworkServer::new(1024 * 1024);
    let mut client = AssetNetworkClient::new(4);

    let id = AssetId::from_content(b"small asset data");
    let data = b"small asset data".to_vec();

    server.register_asset(id, data.clone());
    client.request_asset(id, TransferPriority::Critical);

    // Client sends request
    let request = client.next_request().expect("Should have request");

    // Server responds
    let responses = server.handle_request(request);
    assert_eq!(responses.len(), 1, "Should have single response for small asset");

    // Client receives response
    for response in responses {
        client.handle_message(response).expect("Should handle response");
    }

    // Verify asset received
    let received = client.take_completed(id).expect("Should have completed asset");
    assert_eq!(received, data, "Received data should match sent data");
}

#[test]
fn test_large_asset_chunked_transfer() {
    let chunk_size = 1024;
    let mut server = AssetNetworkServer::new(chunk_size);
    let mut client = AssetNetworkClient::new(4);

    let id = AssetId::from_content(b"large");
    let data = vec![0x42u8; 5000]; // 5KB data, should be 5 chunks

    server.register_asset(id, data.clone());
    client.request_asset(id, TransferPriority::High);

    let request = client.next_request().expect("Should have request");
    let responses = server.handle_request(request);

    // Should have 5 chunks + 1 complete message
    assert_eq!(responses.len(), 6, "Should have 5 chunks + complete");

    // Client receives all chunks
    for response in responses {
        client.handle_message(response).expect("Should handle chunk");
    }

    // Verify complete asset
    let received = client.take_completed(id).expect("Should have completed asset");
    assert_eq!(received.len(), data.len(), "Size should match");
    assert_eq!(received, data, "Data should match");
}

#[test]
fn test_multiple_concurrent_transfers() {
    let mut server = AssetNetworkServer::new(1024 * 1024);
    let mut client = AssetNetworkClient::new(3);

    let id1 = AssetId::from_content(b"asset1");
    let id2 = AssetId::from_content(b"asset2");
    let id3 = AssetId::from_content(b"asset3");

    let data1 = b"asset1".to_vec();
    let data2 = b"asset2".to_vec();
    let data3 = b"asset3".to_vec();

    server.register_asset(id1, data1.clone());
    server.register_asset(id2, data2.clone());
    server.register_asset(id3, data3.clone());

    client.request_asset(id1, TransferPriority::Critical);
    client.request_asset(id2, TransferPriority::High);
    client.request_asset(id3, TransferPriority::Normal);

    // Process all transfers
    while let Some(request) = client.next_request() {
        let responses = server.handle_request(request);
        for response in responses {
            client.handle_message(response).expect("Should handle response");
        }
    }

    // Verify all assets received
    assert_eq!(client.take_completed(id1).unwrap(), data1);
    assert_eq!(client.take_completed(id2).unwrap(), data2);
    assert_eq!(client.take_completed(id3).unwrap(), data3);
}

#[test]
fn test_priority_ordering() {
    let mut client = AssetNetworkClient::new(10);

    let low = AssetId::from_content(b"low");
    let normal = AssetId::from_content(b"normal");
    let high = AssetId::from_content(b"high");
    let critical = AssetId::from_content(b"critical");

    // Add in random order
    client.request_asset(normal, TransferPriority::Normal);
    client.request_asset(critical, TransferPriority::Critical);
    client.request_asset(low, TransferPriority::Low);
    client.request_asset(high, TransferPriority::High);

    // Should get critical first
    let req = client.next_request().unwrap();
    if let AssetNetworkMessage::Request { asset_id, .. } = req {
        assert_eq!(asset_id, critical);
    } else {
        panic!("Expected Request");
    }

    // Then high
    let req = client.next_request().unwrap();
    if let AssetNetworkMessage::Request { asset_id, .. } = req {
        assert_eq!(asset_id, high);
    } else {
        panic!("Expected Request");
    }

    // Then normal
    let req = client.next_request().unwrap();
    if let AssetNetworkMessage::Request { asset_id, .. } = req {
        assert_eq!(asset_id, normal);
    } else {
        panic!("Expected Request");
    }

    // Finally low
    let req = client.next_request().unwrap();
    if let AssetNetworkMessage::Request { asset_id, .. } = req {
        assert_eq!(asset_id, low);
    } else {
        panic!("Expected Request");
    }
}

#[test]
fn test_resumable_download() {
    let chunk_size = 100;
    let mut server = AssetNetworkServer::new(chunk_size);
    let mut client = AssetNetworkClient::new(4);

    let id = AssetId::from_content(b"resumable");
    let data = vec![0x42u8; 250];

    server.register_asset(id, data.clone());

    // Start download
    client.request_asset(id, TransferPriority::Critical);
    let request = client.next_request().unwrap();
    let responses = server.handle_request(request);

    // Receive only first 2 chunks (simulate interruption)
    for response in responses.iter().take(2) {
        client.handle_message(response.clone()).expect("Should handle chunk");
    }

    // Verify partial progress
    if let Some(TransferStatus::InProgress { bytes_received, .. }) = client.status(&id) {
        assert_eq!(*bytes_received, 200);
    } else {
        panic!("Expected InProgress status");
    }

    // Create a new client to simulate reconnecting after interruption
    let mut new_client = AssetNetworkClient::new(4);
    // Copy the partial data to simulate local cache
    new_client
        .chunk_buffers
        .insert(id, client.chunk_buffers.get(&id).unwrap().clone());

    // Resume download with new client
    new_client.request_asset(id, TransferPriority::Critical);
    let resume_request = new_client.next_request().unwrap();

    // Verify resume offset
    if let AssetNetworkMessage::Request { resume_offset, .. } = resume_request {
        assert_eq!(resume_offset, Some(200));
    } else {
        panic!("Expected Request with resume offset");
    }

    // Server sends remaining chunks
    let resume_responses = server.handle_request(resume_request);

    // New client receives remaining chunks
    for response in resume_responses {
        new_client.handle_message(response).expect("Should handle chunk");
    }

    // Verify complete
    let received = new_client.take_completed(id).expect("Should have completed asset");
    assert_eq!(received, data);
}

#[test]
fn test_checksum_validation_failure() {
    let mut client = AssetNetworkClient::new(4);
    let id = AssetId::from_content(b"test");

    let data = b"test data".to_vec();
    let bad_checksum = [0u8; 32]; // Wrong checksum

    let msg = AssetNetworkMessage::Response {
        asset_id: id,
        data,
        checksum: bad_checksum,
        compressed: false,
    };

    let result = client.handle_message(msg);
    assert!(result.is_err(), "Should fail checksum validation");

    // Asset should not be completed
    assert!(client.take_completed(id).is_none());
}

#[test]
fn test_compressed_transfer() {
    let mut server = AssetNetworkServer::new(1024 * 1024);
    let mut client = AssetNetworkClient::new(4);

    let id = AssetId::from_content(b"compressible");
    // Highly compressible data
    let data = b"This is a test string that should compress well. ".repeat(100);

    server.register_asset(id, data.clone());
    client.request_asset(id, TransferPriority::Critical);

    let request = client.next_request().unwrap();
    let responses = server.handle_request(request);

    // Check if compression was used
    if let AssetNetworkMessage::Response { compressed, data: resp_data, .. } = &responses[0] {
        if *compressed {
            // Compressed data should be smaller
            assert!(resp_data.len() < data.len());
        }
    }

    // Client should still receive correct data
    for response in responses {
        client.handle_message(response).expect("Should handle compressed response");
    }

    let received = client.take_completed(id).unwrap();
    assert_eq!(received, data);
}

#[test]
fn test_asset_not_found() {
    let server = AssetNetworkServer::new(1024 * 1024);
    let mut client = AssetNetworkClient::new(4);

    let id = AssetId::from_content(b"nonexistent");
    client.request_asset(id, TransferPriority::Critical);

    let request = client.next_request().unwrap();
    let responses = server.handle_request(request);

    assert_eq!(responses.len(), 1);

    // Should get error message
    if let AssetNetworkMessage::Error { error, .. } = &responses[0] {
        assert_eq!(error, "Asset not found");
    } else {
        panic!("Expected Error message");
    }

    // Client should handle error
    for response in responses {
        let result = client.handle_message(response);
        assert!(result.is_err());
    }

    // Status should be Failed
    if let Some(TransferStatus::Failed { error }) = client.status(&id) {
        assert_eq!(error, "Asset not found");
    } else {
        panic!("Expected Failed status");
    }
}

#[test]
fn test_max_concurrent_limit() {
    let mut client = AssetNetworkClient::new(2);

    let id1 = AssetId::from_content(b"1");
    let id2 = AssetId::from_content(b"2");
    let id3 = AssetId::from_content(b"3");

    client.request_asset(id1, TransferPriority::Critical);
    client.request_asset(id2, TransferPriority::Critical);
    client.request_asset(id3, TransferPriority::Critical);

    // Should get first 2
    assert!(client.next_request().is_some());
    assert!(client.next_request().is_some());

    // Third should be blocked
    assert!(client.next_request().is_none());
    assert_eq!(client.active_count(), 2);
    assert_eq!(client.pending_count(), 1);
}

#[test]
fn test_deduplication() {
    let mut client = AssetNetworkClient::new(4);
    let id = AssetId::from_content(b"test");

    client.request_asset(id, TransferPriority::Critical);
    assert_eq!(client.pending_count(), 1);

    // Duplicate request should be ignored
    client.request_asset(id, TransferPriority::Critical);
    assert_eq!(client.pending_count(), 1);

    // Different priority should also be ignored
    client.request_asset(id, TransferPriority::Low);
    assert_eq!(client.pending_count(), 1);
}

#[test]
fn test_chunked_transfer_sequential() {
    let chunk_size = 100;
    let mut server = AssetNetworkServer::new(chunk_size);
    let mut client = AssetNetworkClient::new(4);

    let id = AssetId::from_content(b"chunked");
    let data = vec![0x42u8; 350]; // 4 chunks

    server.register_asset(id, data.clone());
    client.request_asset(id, TransferPriority::Critical);

    let request = client.next_request().unwrap();
    let responses = server.handle_request(request);

    // Verify chunks are sequential
    let mut expected_offset = 0u64;
    for (i, response) in responses.iter().enumerate() {
        if let AssetNetworkMessage::Chunk { offset, total_size, .. } = response {
            assert_eq!(*offset, expected_offset);
            assert_eq!(*total_size, 350);

            let expected_chunk_size = if i < 3 { 100 } else { 50 };
            expected_offset += expected_chunk_size;
        } else if i == responses.len() - 1 {
            // Last message should be Complete
            assert!(matches!(response, AssetNetworkMessage::Complete { .. }));
        }
    }

    // Process all
    for response in responses {
        client.handle_message(response).expect("Should handle chunk");
    }

    let received = client.take_completed(id).unwrap();
    assert_eq!(received, data);
}

#[test]
fn test_e2e_full_workflow() {
    let mut server = AssetNetworkServer::new(1024);
    let mut client = AssetNetworkClient::new(3);

    // Register multiple assets
    let ids_and_data: Vec<(AssetId, Vec<u8>)> = vec![
        (AssetId::from_content(b"mesh"), vec![0x01u8; 5000]),
        (AssetId::from_content(b"texture"), vec![0x02u8; 10000]),
        (AssetId::from_content(b"shader"), b"shader code".to_vec()),
    ];

    for (id, data) in &ids_and_data {
        server.register_asset(*id, data.clone());
    }

    // Client requests all assets
    client.request_asset(ids_and_data[0].0, TransferPriority::Critical);
    client.request_asset(ids_and_data[1].0, TransferPriority::High);
    client.request_asset(ids_and_data[2].0, TransferPriority::Normal);

    // Process all transfers
    let mut completed = 0;
    while let Some(request) = client.next_request() {
        let responses = server.handle_request(request);
        for response in responses {
            client.handle_message(response).expect("Should handle message");
        }
    }

    // Verify all assets received correctly
    for (id, expected_data) in ids_and_data {
        let received = client.take_completed(id).expect("Should have completed asset");
        assert_eq!(received, expected_data);
        completed += 1;
    }

    assert_eq!(completed, 3);
}

#[test]
fn test_message_serialization_roundtrip() {
    let messages = vec![
        AssetNetworkMessage::Request {
            asset_id: AssetId::from_content(b"test"),
            resume_offset: Some(1024),
        },
        AssetNetworkMessage::Response {
            asset_id: AssetId::from_content(b"test"),
            data: vec![0x42u8; 100],
            checksum: [0u8; 32],
            compressed: true,
        },
        AssetNetworkMessage::Chunk {
            asset_id: AssetId::from_content(b"test"),
            offset: 1024,
            total_size: 5000,
            data: vec![0x42u8; 100],
            compressed: false,
        },
        AssetNetworkMessage::Complete {
            asset_id: AssetId::from_content(b"test"),
            checksum: [0u8; 32],
        },
        AssetNetworkMessage::Error {
            asset_id: AssetId::from_content(b"test"),
            error: "Test error".to_string(),
        },
    ];

    for msg in messages {
        let bytes = bincode::serialize(&msg).expect("Should serialize");
        let deserialized: AssetNetworkMessage =
            bincode::deserialize(&bytes).expect("Should deserialize");
        assert_eq!(msg, deserialized);
    }
}
