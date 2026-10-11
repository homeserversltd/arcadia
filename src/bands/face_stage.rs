#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceStage {
    Vault,
    Pin,
    Console,
}

impl FaceStage {
    fn identity(self) -> &'static str {
        match self {
            Self::Vault => "vault",
            Self::Pin => "pin",
            Self::Console => "console",
        }
    }
}

#[derive(Clone, Copy)]
struct ResolvedFaceStage {
    stage: FaceStage,
    gate: Option<GateRead>,
}

fn resolve_face_stage_sync(headers: axum::http::HeaderMap) -> ResolvedFaceStage {
    let gate = caduceus_gate_read();
    let Some(gate) = gate else {
        return ResolvedFaceStage {
            stage: FaceStage::Console,
            gate: None,
        };
    };
    if gate.vault.unlock_required {
        return ResolvedFaceStage {
            stage: FaceStage::Vault,
            gate: Some(gate),
        };
    }
    let mut stage = FaceStage::Console;
    if gate.pin.pin_required {
        let attended = document_incarnation_from_headers(&headers)
            .zip(attendance_from_headers(&headers))
            .is_some_and(|(document, attendance)| attendance_cached_validate(&document, &attendance));
        if !attended {
            stage = FaceStage::Pin;
        }
    }
    ResolvedFaceStage {
        stage,
        gate: Some(gate),
    }
}

async fn resolve_face_stage(headers: &axum::http::HeaderMap) -> ResolvedFaceStage {
    let headers = headers.clone();
    tokio::task::spawn_blocking(move || resolve_face_stage_sync(headers))
        .await
        .unwrap_or(ResolvedFaceStage {
            stage: FaceStage::Console,
            gate: None,
        })
}

fn project_gate_read(status: &mut ConsoleStatus, gate: Option<GateRead>) {
    if let Some(gate) = gate {
        status.vault = VaultStatus {
            present: gate.vault.present,
            mounted: gate.vault.mounted,
            auto_decrypt_enabled: gate.vault.auto_decrypt_enabled,
            unlock_required: gate.vault.unlock_required,
        };
        status.gui_pin.pin_required = gate.pin.pin_required;
    } else {
        status.vault = VaultStatus {
            present: false,
            mounted: false,
            auto_decrypt_enabled: false,
            unlock_required: false,
        };
        status.gui_pin.pin_required = false;
    }
}

async fn face_stage_guard(
    State(_state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path != "/api" && !path.starts_with("/api/") {
        return next.run(request).await;
    }
    let method = request.method().clone();
    let resolved = resolve_face_stage(request.headers()).await;
    let admitted = match resolved.stage {
        FaceStage::Console => true,
        FaceStage::Vault => {
            method == axum::http::Method::POST && path == "/api/vault/unlock"
        }
        FaceStage::Pin => {
            method == axum::http::Method::POST && path == "/api/v1/exousia/open"
        }
    };
    if admitted {
        if resolved.stage == FaceStage::Console {
            request.extensions_mut().insert(resolved);
        }
        next.run(request).await
    } else {
        match resolved.stage {
            FaceStage::Vault => (
                StatusCode::LOCKED,
                Json(serde_json::json!({"code": "face-stage-vault"})),
            )
                .into_response(),
            FaceStage::Pin => (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"code": "face-stage-pin"})),
            )
                .into_response(),
            FaceStage::Console => unreachable!(),
        }
    }
}
