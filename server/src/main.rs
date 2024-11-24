
use core::f32;
use std::{f32::consts::PI, io::BufWriter, sync::OnceLock};

use anyhow::Result;
use axum::{body::Body, http::{header::CONTENT_TYPE, Request, StatusCode}, response::{IntoResponse, Redirect}, routing::get, Router};
use lyon_geom::{euclid::Transform2D, Angle, Arc};
use rand::Rng;
use raqote::*;

pub const URL: &str = "http://localhost:1474";

#[tokio::main]
async fn main() -> Result<()> {

    let cat = || async move {

        let mut rng = rand::thread_rng();

        let mut dt = DrawTarget::new(400, 256);

        let base_transform = Transform2D::identity()
            .then_scale(1.1 + rng.gen_range(-0.02..0.02), 1.1 + rng.gen_range(-0.02..0.02))
            .then_rotate(Angle::degrees(rng.gen_range(0.0..360.0)))
            .then_translate(Vector::new(
                195. + rng.gen_range(-70.0..70.0),
                124. + rng.gen_range(-45.0..45.0),
            ));

        draw_cat(&mut dt, &base_transform);

        let png = canvas_to_png(dt).unwrap_or_else(|_| Vec::new());

        (
            StatusCode::OK,
            [(CONTENT_TYPE, "image/png")],
            png
        )
    };

    let app = Router::new()
        .route("/cat", get(cat));
        // .fallback(get(routes::error404()));

    // port 1474 is the port for my previous project plus one
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1474")
        .await
        .unwrap();

    println!("unfortunately we are listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

fn draw_head(dt: &mut DrawTarget) {
    let ears = {
        let mut pb = PathBuilder::new();

        let points = (
            (6., -25.),
            (21., -36.),
            (21., -17.)
        );

        pb.move_to(points.0.0, points.0.1);
        pb.line_to(points.1.0, points.1.1);
        pb.line_to(points.2.0, points.2.1);
        pb.close();

        pb.move_to(-points.0.0, points.0.1);
        pb.line_to(-points.1.0, points.1.1);
        pb.line_to(-points.2.0, points.2.1);
        pb.close();

        pb.finish()
    };

    let head = {
        let mut pb = PathBuilder::new();
        ellipse(&mut pb, 0., 0., 25., 24.);
        pb.close();

        pb.finish()
    };

    let eyes = {
        let mut pb = PathBuilder::new();
        ellipse(&mut pb, 9., -7., 3., 3.);
        ellipse(&mut pb, -9., -7., 3., 3.);
        pb.close();

        pb.finish()
    };

    let nose = {
        let mut pb = PathBuilder::new();

        let p_x = 5.;
        let p_y = 5.;
        let c_x = 10.;
        let c_y = -3.;
        let b_x = 1.;
        let b_y = 10.;
        
        pb.move_to(-p_x, p_y);
        pb.cubic_to(-c_x, c_y, c_x, c_y, p_x, p_y);
        pb.cubic_to(b_x, b_y, -b_x, b_y, -p_x, p_y);
        pb.close();

        pb.finish()
    };

    dt.stroke(&ears, &BLACK, &stroke(), &DRAW);
    dt.fill(&ears, &random_color(), &DRAW);
    
    dt.stroke(&head, &BLACK, &stroke(), &DRAW);
    dt.fill(&head, &random_color(), &DRAW);

    dt.fill(&eyes, &BLACK, &DRAW);

    dt.fill(&nose, &BLACK, &DRAW);
}

fn draw_cat(dt: &mut DrawTarget, base: &Transform) {
    let mut rng = rand::thread_rng();

    let tail = {
        let mut pb = PathBuilder::new();

        let (x, y) = (60., 0.);
        
        let sign = if rng.gen::<bool>() { 1. } else { -1. };

        pb.move_to(x, y);
        if rng.gen::<bool>() {
            let scale = rng.gen_range(2.5..3.5);

            pb.cubic_to(
                x + scale*rng.gen_range(12.0..17.0), y + scale*sign*rng.gen_range(0.0..5.0),
                x + scale*rng.gen_range(-5.0..0.0), y + scale*sign*rng.gen_range(10.0..15.0),
                x + scale*rng.gen_range(15.0..25.0), y + scale*sign*rng.gen_range(5.0..15.0),
            );
        } else {
            let scale = rng.gen_range(3.0..4.0);

            pb.quad_to(
                x + scale*rng.gen_range(12.0..17.0), y + scale*sign*rng.gen_range(0.0..5.0),
                x + scale*rng.gen_range(5.0..20.0), y + scale*sign*rng.gen_range(12.0..17.0),
            );
        }

        pb.finish()
    };

    let neck = {
        let mut pb = PathBuilder::new();
        pb.rect(-15., -15., 30., 30.);
        pb.close();

        pb.finish()
    };

    let body = {
        let mut pb = PathBuilder::new();
        ellipse(&mut pb, 0., 0., 60., 27.);
        pb.close();

        pb.finish()
    };

    let leg = {
        let mut pb = PathBuilder::new();

        ellipse(&mut pb, 0., 0., 7., 25.);

        pb.finish()
    };

    dt.set_transform(&base);
    
    dt.stroke(&tail, &BLACK, &StrokeStyle {
        cap: LineCap::Round,
        join: LineJoin::Miter,
        width: 7.,
        miter_limit: 2.,
        dash_array: Vec::new(),
        dash_offset: 0.,
    }, &DRAW);

    dt.set_transform(&Transform::rotation(Angle::degrees(-30.)).then_translate(Vector::new(-45., -19.)).then(&base));
    dt.stroke(&neck, &BLACK, &stroke(), &DRAW);
    dt.fill(&neck, &random_color(), &DRAW);

    let legs = [
        ((-45., 21.), 20.),
        ((-25., 26.), 5.),
        (( 25., 26.), -5.),
        (( 45., 21.), -20.),
    ];

    for ((x, y), rot) in legs {
        let translation = Transform::rotation(Angle::degrees(rot)).then_translate(Vector::new(x, y));

        dt.set_transform(&translation.then(&base));
        dt.stroke(&leg, &BLACK, &stroke(), &DRAW);
        dt.fill(&leg, &random_color(), &DRAW);
    }

    dt.set_transform(&base);
    
    dt.stroke(&body, &BLACK, &stroke(), &DRAW);
    dt.fill(&body, &random_color(), &DRAW);

    // Draw head at (-59, 44).
    dt.set_transform(&Transform::translation(-59., -44.).then(&base));
    draw_head(dt);
    dt.set_transform(&base);


    // Colon three :3
    // {
    //     let path = {
    //         let mut pb = PathBuilder::new();

    //         let (x, y) = (100. + 0., 100. + 7.);
    //         let (rx, ry) = (6., 6.);
    
    //         let a: Arc<f32> = Arc {
    //             center: Point::new(x - rx, y),
    //             radii: Vector::new(rx, ry),
    //             start_angle: Angle::radians(-0.5/3. * PI),
    //             sweep_angle: Angle::radians(4./3. * PI),
    //             x_rotation: Angle::zero(),
    //         };
    //         a.for_each_quadratic_bezier(&mut |q| {
    //             pb.quad_to(q.ctrl.x, q.ctrl.y, q.to.x, q.to.y);
    //             pb.move_to(q.to.x, q.to.y);
    //         });
    //         pb.close();
    
    //         let a: Arc<f32> = Arc {
    //             center: Point::new(x + rx, y),
    //             radii: Vector::new(rx, ry),
    //             start_angle: Angle::radians(-2.5/3. * PI),
    //             sweep_angle: Angle::radians(-4./3. * PI),
    //             x_rotation: Angle::zero(),
    //         };
    //         a.for_each_quadratic_bezier(&mut |q| {
    //             pb.quad_to(q.ctrl.x, q.ctrl.y, q.to.x, q.to.y);
    //             pb.move_to(q.to.x, q.to.y);
    //         });
    //         pb.close();

    //         pb.finish()
    //     };

    //     let style = &StrokeStyle {
    //         cap: LineCap::Round,
    //         join: LineJoin::Miter,
    //         width: 3.,
    //         miter_limit: 2.,
    //         dash_array: Vec::new(),
    //         dash_offset: 0.,
    //     };

    //     dt.stroke(&path, &BLACK, &style, &DRAW);
    // }
}

fn stroke() -> &'static StrokeStyle {
    static STROKE: OnceLock<StrokeStyle> = OnceLock::new();
    STROKE.get_or_init(|| {
        StrokeStyle {
            cap: LineCap::Round,
            join: LineJoin::Miter,
            width: 5.,
            miter_limit: 2.,
            dash_array: Vec::new(),
            dash_offset: 0.,
        }
    })
}

const BLACK: Source = Source::Solid(SolidSource {
    r: 0x0,
    g: 0x0,
    b: 0x0,
    a: 0xff,
});

const DRAW: DrawOptions = DrawOptions {
    blend_mode: BlendMode::SrcOver,
    alpha: 1.,
    antialias: AntialiasMode::Gray,
};

fn random_color<'a>() -> Source<'a> {
    let mut rng = rand::thread_rng();
    Source::Solid(SolidSource {
        r: rng.gen_range(100..=255),
        g: rng.gen_range(100..=255),
        b: rng.gen_range(100..=255),
        a: 0xff,
    })
}

fn ellipse(pb: &mut PathBuilder, x: f32, y: f32, width: f32, height: f32) {
    let a: Arc<f32> = Arc {
        center: Point::new(x, y),
        radii: Vector::new(width, height),
        start_angle: Angle::radians(0.),
        sweep_angle: Angle::radians(std::f32::consts::PI * 2.),
        x_rotation: Angle::zero(),
    };
    let start = a.from();
    pb.move_to(start.x, start.y);
    a.for_each_quadratic_bezier(&mut |q| {
        pb.quad_to(q.ctrl.x, q.ctrl.y, q.to.x, q.to.y);
    });
}

/// Renders a canvas to a PNG.
/// 
/// This is an adaptation of the code in raqote:
/// https://github.com/jrmuizel/raqote/blob/master/src/draw_target.rs#L1096
fn canvas_to_png(canvas: DrawTarget) -> Result<Vec<u8>> {

    let mut file = Vec::new();

    {
        let w = &mut BufWriter::new(&mut file);

        let mut encoder = png::Encoder::new(w, canvas.width() as u32, canvas.height() as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        let buf = canvas.get_data();
        let mut output = Vec::with_capacity(buf.len() * 4);

        for pixel in buf {
            let a = (pixel >> 24) & 0xffu32;
            let mut r = (pixel >> 16) & 0xffu32;
            let mut g = (pixel >> 8) & 0xffu32;
            let mut b = (pixel >> 0) & 0xffu32;

            if a > 0u32 {
                r = r * 255u32 / a;
                g = g * 255u32 / a;
                b = b * 255u32 / a;
            }

            output.push(r as u8);
            output.push(g as u8);
            output.push(b as u8);
            output.push(a as u8);
        }

        writer.write_image_data(&output)?;
    }

    Ok(file)
}