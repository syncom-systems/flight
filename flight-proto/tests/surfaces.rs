// SPDX-License-Identifier: MIT

//! The messages that add and remove surfaces of a workspace (`multiple_surfaces_v1`) survive the
//! wire, validate, and are left alone by a peer that does not know them.

use flight_proto::codec::{decode, encode};
use flight_proto::command_kind::{CreateSurface, Kind, RemoveSurface};
use flight_proto::{
    capability, orchestrator_body, Command, OrchestratorFrame, Request, SurfaceKindCode, Validate,
};
use prost::Message;

fn framed(kind: Kind) -> OrchestratorFrame {
    OrchestratorFrame {
        body: Some(orchestrator_body::Body::Request(Request {
            request_id: 7,
            command: Some(Command { kind: Some(kind) }),
        })),
    }
}

fn round_trip(kind: Kind) -> Kind {
    let frame = framed(kind);
    let back: OrchestratorFrame = decode(&encode(&frame)).unwrap();
    let Some(orchestrator_body::Body::Request(Request {
        command: Some(Command { kind: Some(kind) }),
        ..
    })) = back.body
    else {
        panic!("the command was lost on the wire")
    };
    kind
}

#[test]
fn another_shell_and_a_removal_survive_the_wire() {
    let another = Kind::CreateSurface(CreateSurface {
        workspace_id: "w-abc".into(),
        kind: SurfaceKindCode::Shell as i32,
        another: true,
    });
    assert_eq!(round_trip(another.clone()), another);
    let remove = Kind::RemoveSurface(RemoveSurface {
        workspace_id: "w-abc".into(),
        surface_id: "s-def".into(),
    });
    assert_eq!(round_trip(remove.clone()), remove);
}

#[test]
fn the_earlier_create_surface_is_the_same_bytes_without_another() {
    // A peer that predates the field sends and reads exactly this.
    let old = CreateSurface {
        workspace_id: "w-abc".into(),
        kind: SurfaceKindCode::Shell as i32,
        another: false,
    };
    let mut expected = Vec::new();
    prost::encoding::string::encode(1, &old.workspace_id, &mut expected);
    prost::encoding::int32::encode(2, &old.kind, &mut expected);
    assert_eq!(old.encode_to_vec(), expected);
}

#[test]
fn a_removal_names_a_workspace_and_a_surface_by_valid_ids_and_no_host() {
    let remove = |workspace: &str, surface: &str| Command {
        kind: Some(Kind::RemoveSurface(RemoveSurface {
            workspace_id: workspace.into(),
            surface_id: surface.into(),
        })),
    };
    let good = remove("w-abc", "s-def");
    good.validate().unwrap();
    assert_eq!(good.target_host(), None, "the orchestrator finds the host");
    for (w, s) in [("", "s-1"), ("w-1", ""), ("w:1", "s-1"), ("w-1", "../x")] {
        assert!(remove(w, s).validate().is_err(), "{w:?} {s:?}");
    }
}

#[test]
fn the_capability_is_known_and_negotiated_away_by_a_peer_without_it() {
    assert!(capability::KNOWN.contains(&"multiple_surfaces_v1"));
    let offered = vec![
        "multiple_surfaces_v1".to_owned(),
        "create_surface_v1".to_owned(),
    ];
    assert_eq!(
        capability::negotiate(&offered, &["create_surface_v1"]),
        vec!["create_surface_v1".to_owned()]
    );
}
