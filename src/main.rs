#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, NativeOptions};
use egui::{ComboBox, Grid, RichText};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex};

use core::f64;
use std::f64::consts::TAU;
use regex::Regex;

const NORMAL_TEXT: f32 = 15.0;
const TITLE_TEXT: f32 = 20.0;

fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Create Big Cannons - H's Ballistics Calculator",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

pub fn verify_signed_float_input(s: &mut String) {
    let re = Regex::new(r"^-?[0-9]*\.?[0-9]*").unwrap();
    let cap = re.captures(s);
    if cap.is_none() {
        *s = "".to_string();
    } else {
        *s = re.captures(s).unwrap().get(0).unwrap().as_str().to_string();
    }
}

pub fn verify_positive_integer_input(s: &mut String) {
    let re = Regex::new(r"^[1-9]+[0-9]*").unwrap();
    let cap = re.captures(s);
    if cap.is_none() {
        *s = "".to_string();
    } else {
        *s = re.captures(s).unwrap().get(0).unwrap().as_str().to_string();
    }
}

//function whose roots are the pitch angles for targetting
fn angle_check(x: f64, y: f64, u: f64, v: f64, a: f64, g: f64) -> f64 {
    let p: f64 = (x*u)/(v*a.cos());
    (u*u*x*(a.tan()))/g + p - (y*u*u)/g + (1.0-p).ln()
}

//Find critical point of angle_check through the regula falsi method to get the initial guess for root-finding and selecting direct and indirect shot pitch angles
//Should be able to optimize it better, or use an external math crate if it becomes a problem
fn find_critical_point(x: f64, u: f64, v: f64, g: f64) -> f64{
    let mut a: f64 = (g*x).atan2(v*v);
    let mut b: f64 = (g*x).atan2(-v*v);
    let mut c: f64;

    loop {
        let fa = g*x*a.sin() + u*v*x - v*v*a.cos();
        let fb = g*x*b.sin() + u*v*x - v*v*b.cos();

        c = b - (fb * (b - a)) / (fb - fa);
        
        let fc = g*x*c.sin() + u*v*x - v*v*c.cos();
        if fc.abs() < 0.00001 {
            break
        } else if fc.signum() == fa.signum() {
            a = c;
        } else {
            b = c;
        }
    }

    c
}

//Use the secand method to find the roots of angle_check (Newton's method fails)
//Currently itering until the precision of f64 causes a NaN return, so it could be optimized if that somehow becomes an issue
//Considering moving to the bisection method to ensure convergence
fn find_angles(x: f64, y: f64, u: f64, v: f64, g: f64, critical_point: f64) -> Result<(f64, f64), String>{
    let mut angles: [f64; 2] = [0.0, 0.0];
    
    let cpa = angle_check(x, y, u, v, g, critical_point);
    if cpa < 0.0 {
        return Err("Out of range".to_string());
    } else if cpa < 1e-12 {
        return Ok((cpa, cpa));
    }
    
    for i in 0..2 {
        let mut a: f64 = critical_point;

        let mut b = - 0.011111111 / TAU; // -4°
        if i == 1 { b += TAU/4.0; }
        else { b -= TAU/4.0; }
        
        loop {
            let fb = angle_check(x, y, u, v, b, g);
            if fb < 0.0 { break }
            else {
                if i == 0 { b += 0.0017453292519943296; } // 0.1°
                else { b-= 0.0017453292519943296; }
            }
        }

        let mut c: f64;
        loop {
            let fa = angle_check(x, y, u, v, a, g);
            let fb = angle_check(x, y, u, v, b, g);

            c = b - (fb * (b - a)) / (fb - fa);
            
            let fc = angle_check(x, y, u, v, c, g);
            if fc.abs() < 1e-12 {
                break
            } else if fc.signum() == fa.signum() {
                a = c;
            } else if fc.signum() == fb.signum() {
                b = c;
            } else {
                panic!("Impossible Error (angle_check returned NAN)");
            }
        }
        angles[i] = c;  
    }

    Ok((angles[0], angles[1]))
}

/*
          -X (90°)
             ^
             |
-Z (180°) <--O--> +Z (0°)
             |
             v
          +X (180°)
*/
pub fn calc_yaw(x: f64, z: f64) -> f64 {
    let mut yaw: f64 = -x.atan2(z);
    if yaw < 0.0 { yaw += TAU }
    yaw
}
enum AmmoType {
    Shot,
    APShot,
    APShell,
    HEShell,
    MortarStone,
    SmokeShell,
}

struct Ammo {
    kind: AmmoType,
    drag: f64,
    gravity: f64,
    name: String
}

impl Ammo {
    fn shot() -> Self {
        Self {
            kind: AmmoType::Shot,
            drag: 0.01,
            gravity: 10.0,
            name: "Shot".to_string()
        }
    }
    fn ap_shot() -> Self {
        Self {
            kind: AmmoType::APShot,
            drag: 0.01,
            gravity: 10.0,
            name: "AP Shot".to_string()
        }
    }
    fn ap_shell() -> Self {
        Self {
            kind: AmmoType::APShell,
            drag: 0.01,
            gravity: 10.0,
            name: "AP Shell".to_string()
        }
    }
    fn he_shell() -> Self {
        Self {
            kind: AmmoType::HEShell,
            drag: 0.01,
            gravity: 10.0,
            name: "HE Shell".to_string()
        }
    }
    fn mortar_stone() -> Self {
        Self {
            kind: AmmoType::MortarStone,
            drag: 0.01,
            gravity: 5.0,
            name: "Mortar Stone".to_string()
        }
    }
    fn smoke_shell() -> Self {
        Self {
            kind: AmmoType::SmokeShell,
            drag: 0.01,
            gravity: 10.0,
            name: "Smoke Shell".to_string()
        }
    }

    fn select(ammo_type: &str) -> Ammo {
        match ammo_type {
            "Shot"          => { Ammo::shot() }
            "AP Shot"       => { Ammo::ap_shot() }
            "AP Shell"      => { Ammo::ap_shell() }
            "HE Shell"      => { Ammo::he_shell() }
            "Mortar Stone"  => { Ammo::mortar_stone() }
            "Smoke Shell"   => { Ammo::smoke_shell() }
            _ => {Ammo::shot()}
        }
    }
    
}

impl PartialEq for Ammo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

struct Pair {
    pub direct_shot: f64,
    pub indirect_shot: f64
}

enum MyTabKind {
    Cartesian,
}

struct MyTab {
    kind: MyTabKind,
    surface: SurfaceIndex,
    node: NodeIndex,
    c_x: String,
    c_y: String,
    c_z: String,
    t_x: String,
    t_y: String,
    t_z: String,
    ammo_type: Ammo,
    charges: String,
    yaw: f64,
    pitch: Pair,
    time: Pair,
    impact_angle: Pair,
    nozzle_velocity: String, //Remove after calibration
    drag: String //Remove after calibration
}

impl MyTab {
    fn cartesian(surface: SurfaceIndex, node: NodeIndex) -> Self {
        Self {
            kind: MyTabKind::Cartesian,
            surface,
            node,
            c_x: "".to_string(),
            c_y: "".to_string(),
            c_z: "".to_string(),
            t_x: "".to_string(),
            t_y: "".to_string(),
            t_z: "".to_string(),
            ammo_type: Ammo::shot(),
            charges: "1".to_string(),
            yaw: f64::NAN,
            pitch: Pair {direct_shot: f64::NAN, indirect_shot: f64::NAN},
            time: Pair {direct_shot: f64::NAN, indirect_shot: f64::NAN},
            impact_angle: Pair {direct_shot: f64::NAN, indirect_shot: f64::NAN},
            nozzle_velocity: "".to_string(), //Remove after calibration
            drag: "".to_string() //Remove after calibration
        }
    }

    fn cartesian_tab_content(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Cartesian").size(30.0));
        });

        //Fields for cannon and target coords
        Grid::new("coords")
        .min_col_width(ui.available_width() / 2.0 - 100.0)
        .max_col_width(ui.available_width() / 2.0 - 100.0)
        .min_row_height(15.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                Grid::new("cannon-info")
                .min_col_width(10.0)
                .max_col_width(80.0)
                .min_row_height(15.0)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new(" Cannon").size(TITLE_TEXT));
                    ui.end_row();

                    ui.label(RichText::new("X: ").size(NORMAL_TEXT));
                    if ui.text_edit_singleline(&mut self.c_x).changed() {
                        verify_signed_float_input(&mut self.c_x);
                    }

                    ui.end_row();
                    ui.label(RichText::new("Y: ").size(NORMAL_TEXT));
                    if ui.text_edit_singleline(&mut self.c_y).changed() {
                        verify_signed_float_input(&mut self.c_y);
                    }

                    ui.end_row();
                    ui.label(RichText::new("Z: ").size(NORMAL_TEXT));
                    if ui.text_edit_singleline(&mut self.c_z).changed() {
                        verify_signed_float_input(&mut self.c_z);
                    }
                    ui.end_row();
                    ui.label(RichText::new("  ").size(NORMAL_TEXT));
                });
            });
            ui.vertical(|ui| {
                Grid::new("target-info")
                .min_col_width(10.0)
                .max_col_width(80.0)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new(" Target").size(TITLE_TEXT));
                    ui.end_row();

                    ui.label(RichText::new("X: ").size(NORMAL_TEXT));
                    if ui.text_edit_singleline(&mut self.t_x).changed() {
                        verify_signed_float_input(&mut self.t_x);
                    }

                    ui.end_row();
                    ui.label(RichText::new("Y: ").size(NORMAL_TEXT));
                    if ui.text_edit_singleline(&mut self.t_y).changed() {
                        verify_signed_float_input(&mut self.t_y);
                    }

                    ui.end_row();
                    ui.label(RichText::new("Z: ").size(NORMAL_TEXT));
                    if ui.text_edit_singleline(&mut self.t_z).changed() {
                        verify_signed_float_input(&mut self.t_z);
                    }
                });
            });
        });
        
        //Ammo type selector and number of powder charges
        ui.horizontal(|ui| {
            ComboBox::new("Ammo type", RichText::new(" :Ammo type").size(NORMAL_TEXT))
            .selected_text(RichText::new(format!("{}", self.ammo_type.name)).size(NORMAL_TEXT))
            .show_ui(ui, |ui| {
                for ammo_type in ["Shot", "AP Shot", "AP Shell", "HE Shell", "Mortar Stone", "Smoke Shell"] {
                    ui.selectable_value(
                        &mut self.ammo_type,
                        Ammo::select(ammo_type),
                        RichText::new(ammo_type).size(NORMAL_TEXT)
                    );
                }
            });

            ui.add_space(10.0);

            Grid::new("charges")
            .max_col_width(30.0)
            .show(ui, |ui| {
                if ui.text_edit_singleline(&mut self.charges).changed() {
                    verify_positive_integer_input(&mut self.charges);
                }
            });

            ui.label(RichText::new(" :Powder charges").size(NORMAL_TEXT));

            //Remove after calibration
            Grid::new("velocity")
            .max_col_width(30.0)
            .show(ui, |ui| {
                if ui.text_edit_singleline(&mut self.nozzle_velocity).changed() {
                    verify_signed_float_input(&mut self.nozzle_velocity);
                }
            });
            ui.label(RichText::new(" :Nozzle velocity").size(NORMAL_TEXT));

            Grid::new("velocity")
            .max_col_width(30.0)
            .show(ui, |ui| {
                if ui.text_edit_singleline(&mut self.drag).changed() {
                    verify_signed_float_input(&mut self.drag);
                }
            });
            ui.label(RichText::new(" :Drag").size(NORMAL_TEXT));

        });

        if ui.button(RichText::new("Calculate").size(TITLE_TEXT)).clicked() {
            let mut x: f64 = 0.0;
            let mut y: f64 = 0.0;
            let mut z: f64 = 0.0;

            //Convert input coords of cannon and target to f64 and store the difference

            match self.t_x.parse::<f64>() {
                Ok(t_x) => x += t_x,
                Err(_) => {}
            }
            match self.c_x.parse::<f64>() {
                Ok(t_x) => x -= t_x,
                Err(_) => {}
            }

            match self.t_y.parse::<f64>() {
                Ok(t_y) => y += t_y,
                Err(_) => {}
            }
            match self.c_y.parse::<f64>() {
                Ok(t_y) => y -= t_y,
                Err(_) => {}
            }

            match self.t_z.parse::<f64>() {
                Ok(t_z) => z += t_z,
                Err(_) => {}
            }
            match self.c_z.parse::<f64>() {
                Ok(t_z) => z -= t_z,
                Err(_) => {}
            }

            self.yaw = calc_yaw(x, z);

            //TO-DO: Implement usage of ammo type and ammount of power charges, calibratrion required
            
            //Remove after calibration
            let mut v: f64 = f64::NAN;
            match self.nozzle_velocity.parse::<f64>() {
                Ok(nozzle_velocity) => v = nozzle_velocity,
                Err(_) => {}
            }

            let mut u: f64 = f64::NAN;
            match self.drag.parse::<f64>() {
                Ok(drag) => u = drag,
                Err(_) => {}
            }

            let d: f64 = (x*x + z*z).sqrt();

            let critical_point = find_critical_point(d, u, v, self.ammo_type.gravity);
            let angles = find_angles(d, y, u, v, self.ammo_type.gravity, critical_point);

            match angles {
                Ok(angles) => {
                    self.pitch.direct_shot = angles.0;
                    self.pitch.indirect_shot = angles.1;
                }
                _ => {
                    self.pitch.direct_shot = f64::NAN;
                    self.pitch.indirect_shot = f64::NAN;
                }
            }
        }

        //Show results
        Grid::new("results")
        .min_col_width(ui.available_width() / 2.0)
        .max_col_width(ui.available_width() / 2.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("Direct Shot     ").size(NORMAL_TEXT * (4.0/3.0)));
                    ui.label(RichText::new(format!("Yaw: {:.4}°", self.yaw.to_degrees())).size(NORMAL_TEXT));
                    if self.pitch.direct_shot.is_finite() {
                        ui.label(RichText::new(format!("Pitch: {}°", self.pitch.direct_shot.to_degrees())).size(NORMAL_TEXT));
                        ui.label(RichText::new(format!("Flight time: {:.4}s", self.time.direct_shot)).size(NORMAL_TEXT));
                        ui.label(RichText::new(format!("Impact angle: {:.4}°", self.impact_angle.direct_shot.to_degrees())).size(NORMAL_TEXT));
                    } else {
                        ui.label(RichText::new("OUT OF RANGE").size(NORMAL_TEXT * (4.0/3.0)));
                    }
                });
            });
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.label(RichText::new("Indirect Shot   ").size(NORMAL_TEXT * (4.0/3.0)));
                    ui.label(RichText::new(format!("Yaw: {:.4}°", self.yaw.to_degrees())).size(NORMAL_TEXT));
                    if self.pitch.direct_shot.is_finite() {
                        ui.label(RichText::new(format!("Pitch: {}°", self.pitch.indirect_shot.to_degrees())).size(NORMAL_TEXT));
                        ui.label(RichText::new(format!("Flight time: {:.4}s", self.time.indirect_shot)).size(NORMAL_TEXT));
                        ui.label(RichText::new(format!("Impact angle: {:.4}°", self.impact_angle.indirect_shot.to_degrees())).size(NORMAL_TEXT));
                    } else {
                        ui.label(RichText::new("OUT OF RANGE").size(NORMAL_TEXT * (4.0/3.0)));
                    }
                });
            });
        });
    }

    fn title(&self) -> String {
        match self.kind {
            MyTabKind::Cartesian => format!("Cartesian Tab {}", self.node.0),
        }
    }
}
struct TabViewer<'a> {
    added_nodes: &'a mut Vec<MyTab>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = MyTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.cartesian_tab_content(ui);
    }

    fn add_popup(&mut self, ui: &mut egui::Ui, surface: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(80.0);
        ui.style_mut().visuals.button_frame = false;

        if ui.button("Cartesian tab").clicked() {
            self.added_nodes.push(MyTab::cartesian(surface, node));
        }
    }
}

struct MyApp {
    dock_state: DockState<MyTab>,
    counter: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        let tree = DockState::new(vec![
            MyTab::cartesian(SurfaceIndex::main(), NodeIndex(1)),
        ]);

        Self {
            dock_state: tree,
            counter: 2,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut added_nodes = Vec::new();
        DockArea::new(&mut self.dock_state)
            .show_add_buttons(true)
            .show_add_popup(true)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show(
                ctx,
                &mut TabViewer {
                    added_nodes: &mut added_nodes,
                },
            );
        
        added_nodes.drain(..).for_each(|node| {
            self.dock_state
                .set_focused_node_and_surface((node.surface, node.node));
            self.dock_state.push_to_focused_leaf(MyTab {
                kind: node.kind,
                surface: node.surface,
                node: NodeIndex(self.counter),
                c_x: node.c_x,
                c_y: node.c_y,
                c_z: node.c_z,
                t_x: node.t_x,
                t_y: node.t_y,
                t_z: node.t_z,
                ammo_type: node.ammo_type,
                charges: node.charges,
                yaw: node.yaw,
                pitch: node.pitch,
                time: node.time,
                impact_angle: node.impact_angle,
                nozzle_velocity: node.nozzle_velocity, //Remove after calibration
                drag: node.drag //Remove after calibration
            });
            self.counter += 1;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //pre-calculated data set
    //x, y, u, v, g, a, t
    const TESTING_DATA: [[f64; 7]; 8] = [
        [   23.541096135,    0.959446698, 0.01,  30.0, 10.0,  0.174532925, 0.8 ],
        [  187.001956030,   63.079770828, 0.01, 200.0, 10.0,  0.349065850, 1.0 ],
        [   64.467192584,   26.026190686, 0.01,  50.0, 10.0,  0.523598776, 1.5 ],
        [ 1132.001739726,  905.308887445, 0.01, 500.0, 10.0,  0.698131701, 3.0 ],
        [ 1709.752036132, 1993.049776655, 0.01, 900.0, 10.0,  0.872664626, 3.0 ],
        [   54.698606123,   88.712887372, 0.01, 100.0, 10.0,  1.047197551, 1.1 ],
        [  249.003450881,  -58.274490171, 0.01, 150.0, 10.0, -0.174532925, 1.7 ],
        [   28.120418992,  -11.482914756, 0.01,  60.0, 10.0, -0.349065850, 0.5 ],
    ];

    #[test]
    fn angle_calculation() {
        for i in TESTING_DATA {
            let crit = find_critical_point(i[0], i[2], i[3], i[4]);
            let angles = find_angles(i[0], i[1], i[2], i[3], i[4], crit);

            match angles {
                Ok(angle) => {
                    if ! ( (0.00001 > (angle.1 - i[5]).abs()) || (0.00001 > (angle.0 - i[5]).abs())) {
                        panic!("Failiure on test conditions {} {} {} {} {} {} {}, got crit {} and angles {} {}", i[0], i[1], i[2], i[3], i[4], i[5], i[6], crit, angle.0, angle.1)
                    }
                }
                _ => {panic!("Unexpected outcome, find_angles didn't return anything")} //May change
            }
        }
    }

}