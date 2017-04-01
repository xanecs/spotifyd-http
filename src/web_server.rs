use nickel::{Nickel, HttpRouter, FormBody, Request, Response, MiddlewareResult};
use nickel::status::StatusCode;

use librespot::spirc::SpircManager;
use librespot::util::SpotifyId;

use rustc_serialize::json::ToJson;

fn ok<T>(_: T) -> StatusCode {
    StatusCode::Ok
}

fn enable_cors<'mw>(_req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    res.headers_mut()
       .set_raw("Access-Control-Allow-Origin", vec![b"*".to_vec()]);
    res.headers_mut()
       .set_raw("Access-Control-Allow-Headers",
                vec![b"Origin X-Requested-With Content-Type Accept".to_vec()]);
    res.next_middleware()
}

pub fn run(spirc: SpircManager) {
    let mut server = Nickel::new();

    server.utilize(enable_cors);

    let spirc_device_list = spirc.clone();
    server.get("/devices",
               middleware!(spirc_device_list.devices().to_json()));

    let spirc_get_tracks = spirc.clone();
    server.get("/:device/tracks",
               middleware! { |req, res|
        if let Some(tracks) = spirc_get_tracks.device_tracks(req.param("device").unwrap()) {
            return res.send(tracks.ids
                                  .iter()
                                  .map(SpotifyId::to_base62)
                                  .collect::<Vec<_>>().to_json());
        }

        (StatusCode::NotFound, "No tracks for that device id.")
    });

    let spirc_get_track = spirc.clone();
    server.get("/:device/track",
               middleware! { |req, res|
        if let Some(tracks) = spirc_get_track.device_tracks(req.param("device").unwrap()) {
            return res.send(SpotifyId::to_base62(&tracks.ids[(tracks.index as usize)]));
        }

        (StatusCode::NotFound, "No track for that device id.")
    });

    let spirc_put_tracks = spirc.clone();
    server.put("/:device/tracks",
               middleware! { |req, res|
        let device = req.param("device").unwrap().to_owned();
        let form_data = try_with!(res, req.form_body());

        if let Some(id_strs) = form_data.all("id") {
            let tracks = id_strs.iter().map(|id| SpotifyId::from_base62(&*id));
            spirc_put_tracks.clone().send_replace_queue(&*device, tracks);
            StatusCode::Ok
        } else {
            StatusCode::BadRequest
        }
    });

    let spirc_post_tracks = spirc.clone();
    server.post("/:device/tracks",
                middleware! { |req, res|
        let device = req.param("device").unwrap().to_owned();
        let form_data = try_with!(res, req.form_body());

        if let Some(id_strs) = form_data.all("id") {
            let tracks = id_strs.iter().map(|id| SpotifyId::from_base62(&*id));
            spirc_post_tracks.clone().send_append_tracks(&*device, tracks);
            StatusCode::Ok
        } else {
            StatusCode::BadRequest
        }
    });

    let spirc_cmd = spirc.clone();
    server.put("/:device/:cmd",
               middleware!(|req| {
        match req.param("cmd") {
            Some("pause") => ok(spirc_cmd.send_pause(req.param("device").unwrap())),
            Some("play") => ok(spirc_cmd.send_play(req.param("device").unwrap())),
            Some("next") => ok(spirc_cmd.send_next(req.param("device").unwrap())),
            Some("prev") => ok(spirc_cmd.send_prev(req.param("device").unwrap())),
            _ => StatusCode::NotFound,
        }
    }));

    server.listen("0.0.0.0:6767");
}
