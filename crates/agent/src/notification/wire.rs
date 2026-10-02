//! Transport-neutral v3 notification control/event bindings.
use crosslab_core::EventSubscription;
use crosslab_policy::{
    CapabilityId, CapabilityVersion, CapabilityVersionRange, LocalCapability, OperationName,
};
use crosslab_protocol::{
    CapabilityAdvertisement, CapabilityAdvertisementEntry, ControlRequest, ControlResponseResult,
    Event, EventId, EventScope, EventType, NOTIFICATION_CAPABILITY_ID,
    NOTIFICATION_POSTED_EVENT_TYPE, NOTIFICATION_REMOVED_EVENT_TYPE,
    NOTIFICATION_SUBSCRIBE_OPERATION, NotificationPayload, NotificationProfileError,
    ProtocolDiagnostic, ProtocolErrorCode, ProtocolFailure, RequestId, RetryClass,
    validate_notification_subscribe,
};

const VERSION: CapabilityVersion = CapabilityVersion::new(3, 0);

fn capability() -> CapabilityId {
    CapabilityId::parse(NOTIFICATION_CAPABILITY_ID).expect("canonical notification capability")
}

fn event_type(name: &str) -> EventType {
    EventType::parse(name).expect("canonical notification event type")
}

pub(crate) fn notification_capabilities(available: bool) -> Vec<LocalCapability> {
    if !available {
        return Vec::new();
    }
    vec![LocalCapability::new(
        capability(),
        CapabilityVersionRange::new(3, 0, 0).expect("notification v3 range"),
        true,
    )]
}

pub(crate) fn notification_advertisement(available: bool) -> CapabilityAdvertisement {
    let entries = if available {
        vec![
            CapabilityAdvertisementEntry::new(capability(), VERSION, VERSION, true)
                .expect("notification v3 advertisement"),
        ]
    } else {
        Vec::new()
    };
    CapabilityAdvertisement::new(entries).expect("bounded notification advertisement")
}

pub(crate) fn notification_subscribe_request(id: RequestId) -> ControlRequest {
    ControlRequest::new(
        id,
        capability(),
        VERSION,
        OperationName::parse(NOTIFICATION_SUBSCRIBE_OPERATION)
            .expect("canonical notification operation"),
        RetryClass::NonRetryable,
        Vec::new(),
    )
}

fn failure(code: ProtocolErrorCode) -> ControlResponseResult {
    ControlResponseResult::Error(ProtocolFailure::new(
        code,
        Some(
            ProtocolDiagnostic::new("notification subscription not available")
                .expect("bounded fixed diagnostic"),
        ),
    ))
}

pub(crate) fn validate_notification_request(
    request: &ControlRequest,
) -> Result<(), ControlResponseResult> {
    if request.capability_id().as_str() != NOTIFICATION_CAPABILITY_ID {
        return Err(failure(ProtocolErrorCode::CapabilityUnsupported));
    }
    if request.capability_version() != VERSION {
        return Err(failure(ProtocolErrorCode::CapabilityVersionIncompatible));
    }
    if request.operation_name().as_str() != NOTIFICATION_SUBSCRIBE_OPERATION
        || request.retry_class() != RetryClass::NonRetryable
        || validate_notification_subscribe(request.body()).is_err()
    {
        return Err(failure(ProtocolErrorCode::OperationMismatch));
    }
    Ok(())
}

pub(crate) fn validate_notification_response(result: &ControlResponseResult) -> bool {
    matches!(result, ControlResponseResult::Success(bytes) if bytes.is_empty())
}

pub(crate) fn notification_event_subscriptions() -> [EventSubscription; 2] {
    [
        EventSubscription::new(capability(), event_type(NOTIFICATION_POSTED_EVENT_TYPE)),
        EventSubscription::new(capability(), event_type(NOTIFICATION_REMOVED_EVENT_TYPE)),
    ]
}

pub(crate) fn notification_event(
    payload: &NotificationPayload,
) -> Result<Event, NotificationProfileError> {
    let event_id = EventId::generate().map_err(|_| NotificationProfileError::RandomUnavailable)?;
    let kind = match payload {
        NotificationPayload::Posted(_) => NOTIFICATION_POSTED_EVENT_TYPE,
        NotificationPayload::Removed(_) => NOTIFICATION_REMOVED_EVENT_TYPE,
    };
    Event::capability(event_id, capability(), event_type(kind), payload.encode())
        .map_err(|_| NotificationProfileError::Malformed)
}

pub(crate) fn parse_notification_event(
    event: &Event,
) -> Result<Option<NotificationPayload>, NotificationProfileError> {
    let EventScope::Capability(id) = event.scope() else {
        return Ok(None);
    };
    if id.as_str() != NOTIFICATION_CAPABILITY_ID {
        return Ok(None);
    }
    let is_posted = match event.event_type().as_str() {
        NOTIFICATION_POSTED_EVENT_TYPE => true,
        NOTIFICATION_REMOVED_EVENT_TYPE => false,
        _ => return Err(NotificationProfileError::Malformed),
    };
    let payload = NotificationPayload::decode(event.body())?;
    if matches!(payload, NotificationPayload::Posted(_)) != is_posted {
        return Err(NotificationProfileError::Malformed);
    }
    Ok(Some(payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crosslab_protocol::{NotificationId, NotificationPosted};

    #[test]
    fn subscriptions_are_empty_nonretryable_and_exact_v3() {
        let request = notification_subscribe_request(RequestId::from_bytes([1; 16]));
        assert!(validate_notification_request(&request).is_ok());
        assert!(!validate_notification_response(
            &ControlResponseResult::Success(vec![0])
        ));
        assert!(validate_notification_response(
            &ControlResponseResult::Success(Vec::new())
        ));
        let malformed = ControlRequest::new(
            request.request_id(),
            capability(),
            VERSION,
            OperationName::parse(NOTIFICATION_SUBSCRIBE_OPERATION).unwrap(),
            RetryClass::Idempotent,
            Vec::new(),
        );
        assert!(validate_notification_request(&malformed).is_err());
        let extra = ControlRequest::new(
            request.request_id(),
            capability(),
            VERSION,
            OperationName::parse(NOTIFICATION_SUBSCRIBE_OPERATION).unwrap(),
            RetryClass::NonRetryable,
            vec![1],
        );
        assert!(validate_notification_request(&extra).is_err());
    }

    #[test]
    fn event_scope_type_and_version_are_bounded() {
        let p = NotificationPayload::Posted(
            NotificationPosted::new(
                NotificationId::from_bytes([2; 16]),
                "Messages".to_owned(),
                None,
                None,
                true,
            )
            .unwrap(),
        );
        let event = notification_event(&p).unwrap();
        assert_eq!(parse_notification_event(&event), Ok(Some(p)));
        let mismatched = Event::capability(
            EventId::from_bytes([4; 16]),
            capability(),
            event_type(NOTIFICATION_REMOVED_EVENT_TYPE),
            event.body().to_vec(),
        )
        .unwrap();
        assert_eq!(
            parse_notification_event(&mismatched),
            Err(NotificationProfileError::Malformed)
        );
        assert!(notification_capabilities(false).is_empty());
        assert!(notification_advertisement(false).entries().is_empty());
        assert_eq!(notification_event_subscriptions().len(), 2);
    }
}
