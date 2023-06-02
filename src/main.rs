use std::fs::OpenOptions;
use std::io::prelude::*;
use std::process::Command;
use std::fmt;

struct ExtlinuxEntry {
    label: Option<String>,
    menu_label: Option<String>,
    linux: Option<String>,
    fdt: Option<String>,
    initrd: Option<String>,
    append: Option<String>,
}

impl ExtlinuxEntry {
    fn init(&mut self, content: &[&str]) {
        for line in content {
            if line.starts_with("LABEL") {
                let label_string = line.strip_prefix("LABEL").unwrap().trim().to_string();
                self.label = Some(label_string);
            }

            if line.trim().starts_with("MENU LABEL") {
                let menu_label_string = line.trim().strip_prefix("MENU LABEL").unwrap().trim().to_string();
                self.menu_label = Some(menu_label_string);
            }

            if line.trim().starts_with("LINUX") {
                let linux_string = line.trim().strip_prefix("LINUX").unwrap().trim().to_string();
                self.linux = Some(linux_string);
            }

            if line.trim().starts_with("FDT") {
                let fdt_string = line.trim().strip_prefix("FDT").unwrap().trim().to_string();
                self.fdt = Some(fdt_string);
            }

            if line.trim().starts_with("INITRD") {
                let initrd_string = line.trim().strip_prefix("INITRD").unwrap().trim().to_string();
                self.initrd = Some(initrd_string);
            }

            if line.trim().starts_with("APPEND") {
                let append_string = line.trim().strip_prefix("APPEND").unwrap().trim().to_string();
                self.append = Some(append_string);
            }
        }
    }
}

struct Extlinux {
    timeout: Option<usize>,
    default: Option<String>,
    menu_title: Option<String>,
    entries: Vec<ExtlinuxEntry>,
}

impl Extlinux {
    fn init(&mut self) {
        let extlinux_path = String::from("/boot/extlinux/extlinux.conf");
        let mut extlinux = OpenOptions::new()
                                        .read(true)
                                        .create_new(false)
                                        .truncate(false)
                                        .write(true)
                                        .open(&extlinux_path)
                                        .expect("Error : Cannot open /boot/extlinux/extlinux.conf... Please run this program as superuser");
        
        let mut extlinux_content = String::new();
        extlinux.read_to_string(&mut extlinux_content).expect("Error : Cannot read from extlinux");
        let lines = extlinux_content.lines();

        for line in lines {
            if line.starts_with("TIMEOUT") {
                let timeout_string = line.strip_prefix("TIMEOUT").unwrap().trim().to_string();
                self.timeout = Some(timeout_string.parse::<usize>().unwrap());
            }

            if line.starts_with("DEFAULT") {
                let default_string = line.strip_prefix("DEFAULT").unwrap().trim().to_string();
                self.default = Some(default_string);
            }

            if line.starts_with("MENU TITLE") {
                let menu_title_string = line.strip_prefix("MENU TITLE").unwrap().trim().to_string();
                self.menu_title = Some(menu_title_string);
            }
        }

        let mut label_lines = Vec::<usize>::new();
        for (idx, line) in extlinux_content.lines().enumerate() {
            if line.starts_with("LABEL") {
                label_lines.push(idx);
            }
        }
        label_lines.push(0);

        label_lines.into_iter().fold(0, |acc, x| {
            if acc > 0 {
                let mut entry = ExtlinuxEntry {
                    label: None,
                    menu_label: None,
                    linux: None,
                    fdt: None,
                    initrd: None,
                    append: None,
                };
                if x == 0 {
                    entry.init(&extlinux_content.lines().collect::<Vec::<&str>>()[acc..]);
                } else {
                    entry.init(&extlinux_content.lines().collect::<Vec::<&str>>()[acc..x]);
                }
                self.entries.push(entry);
            }
            x
        });

    }
}

struct DtbProperty {
    key: String,
    value: Option<String>,
}

impl fmt::Debug for DtbProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DtbProperty")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish()
    }
}

struct DtbNode {
    node_name: String,
    properties: Vec<DtbProperty>,
    child_nodes: Vec<Box<DtbNode>>,
}

impl fmt::Debug for DtbNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DtbNode")
            .field("node_name", &self.node_name)
            .field("properties", &self.properties)
            .field("child_nodes", &self.child_nodes)
            .finish()
    }
}

impl DtbNode {
    fn indent(n: usize) -> String {
        "\t".repeat(n)
    }

    fn stringify(&self, indent: usize) -> String {
        let mut ret = String::new();

        if self.node_name == "/" && indent == 0 {
            ret.push_str("/dts-v1/;");
            ret.push('\n');
            ret.push('\n');
        }

        ret.push_str(&(Self::indent(indent) + &self.node_name + " {\n")[..]);

        // properties
        for property in &self.properties {
            let mut property_string = Self::indent(indent+1);
            property_string.push_str(&property.key);
            match &property.value {
                Some(value) => {
                    property_string.push_str(" = ");
                    property_string.push_str(&value);
                },
                _ => {}
            }
            property_string.push_str(";\n");

            ret.push_str(&property_string);
        }

        // add new line between properties and child_nodes
        // if there are no child nodes, just close node.
        if self.child_nodes.len() > 0 {
            ret.push_str("\n");
        }

        // child_nodes
        for (idx, node) in self.child_nodes.iter().enumerate() {
            ret.push_str(&node.stringify(indent+1));
            if idx < self.child_nodes.len() - 1 {
                ret.push_str("\n");
            }
        }

        // close node
        ret.push_str(&Self::indent(indent));
        ret.push_str("};\n");

        ret
    }

    fn parse(content_vec: &Vec<&str>, brackets: &Vec<(usize, usize)>, index: usize, start: (usize, usize)) -> (usize, DtbNode) {
        let mut ret = DtbNode {
            node_name: String::new(),
            properties: Vec::<DtbProperty>::new(),
            child_nodes: Vec::<Box<DtbNode>>::new(),
        };

        ret.node_name = content_vec[start.0].trim().strip_suffix("{").unwrap().trim().to_string();

        let mut idx = start.0;
        let mut bracket_idx = index;

        loop {
            idx += 1;
            
            if brackets[bracket_idx + 1].0 == idx && brackets[bracket_idx + 1].1 == 0 {
                // open new child node
                let child: DtbNode;
                let result = Self::parse(content_vec, brackets, bracket_idx + 1, brackets[bracket_idx + 1].clone());
                bracket_idx = result.0;
                idx = brackets[bracket_idx].0;
                child = result.1;

                ret.child_nodes.push(Box::new(child));

            } else if brackets[bracket_idx + 1].0 == idx && brackets[bracket_idx + 1].1 == 1 {
                // close
                return (bracket_idx + 1, ret);
            } else if content_vec[idx].contains(" = ") {
                // property with (key, value)
                let v: Vec<&str> = content_vec[idx].trim().split(" = ").map(|s| s.trim()).collect();
                let property = DtbProperty {
                    key: String::from(v[0]),
                    value: Some(String::from(v[1].trim().strip_suffix(";").unwrap())),
                };

                ret.properties.push(property);
            } else if !content_vec[idx].contains("=") && content_vec[idx].contains(";") {
                // property with no value
                let property = DtbProperty {
                    key: String::from(content_vec[idx].trim().strip_suffix(";").unwrap()),
                    value: None,
                };
                ret.properties.push(property);
            }
            
        }

    }

    fn find_property(&mut self, key: &str) -> Option<&mut DtbProperty> {
        self.properties.iter_mut().find(|property| property.key == key)
    }

    fn find_childnode(&mut self, name: &str) -> Option<&mut Box<DtbNode>> {
        self.child_nodes.iter_mut().find(|node| node.node_name == name)
    }

    fn init(&mut self, content: String) {

        // find open, close brackets
        let content_vec: Vec<&str> = content.lines().collect::<Vec<&str>>();

        let mut opens: Vec<(usize, usize)> = content_vec.iter()
                                                    .enumerate()
                                                    .filter(|(_idx, line)| line.contains("{") && !line.trim().starts_with("//"))
                                                    .map(|(idx, _line)| (idx, 0))
                                                    .collect();
        let mut closes: Vec<(usize, usize)> = content_vec.iter()
                                                    .enumerate()
                                                    .filter(|(_idx, line)| line.contains("};") && !line.trim().starts_with("//"))
                                                    .map(|(idx, _line)| (idx, 1))
                                                    .collect();

        opens.append(&mut closes);
        opens.sort_unstable_by(|(idx1, _), (idx2, _)| idx1.cmp(idx2));

        *self = Self::parse(&content_vec, &opens, 0, opens[0].clone()).1;
    }
}

fn main() {
    let mut extlinux = Extlinux {
        timeout: None,
        default: None,
        menu_title: None,
        entries: Vec::<ExtlinuxEntry>::new(),
    };
    extlinux.init();

/*
    for entry in &extlinux.entries {
        println!("{}", entry.label.clone().unwrap());
        println!("{}", entry.menu_label.clone().unwrap());
        println!("{}", entry.linux.clone().unwrap());
        println!("{}", entry.fdt.clone().unwrap());
        println!("{}", entry.initrd.clone().unwrap());
        println!("{}", entry.append.clone().unwrap());
    }
*/

    // target device tree file name

    let default_entry = extlinux.entries.iter().find(|&entry| entry.label == extlinux.default).unwrap();

    let target_dtb = default_entry.fdt.clone().unwrap();
    let target_dts = target_dtb.as_str().strip_suffix(".dtb").unwrap().to_string() + ".dts";
    let new_dts_filename = target_dtb.as_str().strip_suffix(".dtb").unwrap().to_string() + "_new.dts";
    let new_dtb_filename = new_dts_filename.as_str().strip_suffix(".dts").unwrap().to_string() + ".dtb";
    
    // backup dtb
    print!("Backup device tree blob file... ");

    match std::fs::copy(target_dtb.clone(), target_dtb.clone() + ".backup") {
        Ok(_) => {
            println!("OK");
        },
        _ => {
            panic!("Error : Cannot make dtb backup...");
        },
    }

    print!("Decompiling device tree blob file... ");
    // decompile
    let decompile = Command::new("dtc")
                            .args(["-I", "dtb", "-O", "dts", &target_dtb, "-o", &target_dts])
                            .output()
                            .expect("Error : Cannot decompile dtb file");

    match String::from_utf8(decompile.stderr).unwrap().find("No such file or directory") {
        Some(_) => {
            println!("");
            panic!("Error : Cannot decompile dtb file... No such file or directory");
        },
        _ => {
            println!("OK")
        }
    }
    
    // patch sdcard
    print!("Opening decompiled dts file... ");
    let mut dts = OpenOptions::new()
                        .read(true)
                        .create_new(false)
                        .truncate(false)
                        .open(&target_dts)
                        .expect("Error : Cannot open decompiled dts file");
    println!("OK");

    print!("Reading from opened dts file... ");
    let mut buffer = String::new();
    dts.read_to_string(&mut buffer).expect("Error : Cannot read from dts file");
    println!("OK");
    
    /*
    print!("Finding target node from dts file... ");
    match buffer.find("sdhci@3440000 {") {
        Some(idx) => {
            println!("OK");
            let mut lower = buffer[idx..].to_string();

            print!("Finding status of target node... ");
            let first_status = lower.find("status = ").unwrap();
            let first_disabled_status = lower.find("status = \"disabled\"").unwrap();

            if first_status == first_disabled_status {
                // need to be patched
                println!("OK");
                print!("Patching... ");
                lower = lower.replacen("disabled", "okay", 1);

                let patched_string = buffer[0..idx].to_string() + &lower[..];

                let mut dts_write = OpenOptions::new()
                                        .write(true)
                                        .truncate(true)
                                        .open(&target_dts)
                                        .expect("Error : Cannot open dts file with writeonly");
                
                dts_write.write_all(patched_string.as_bytes()).unwrap();
                println!("OK");
            }
            else
            {
                // already patched
                println!("");
                println!("It seems to be already patched... setup abort");
                return;
            }

        },
        None => {
            println!("microSD patch passed");
        }
    }
    */

    // initialize root node
    let mut root = DtbNode {
        node_name: String::new(),
        properties: Vec::<DtbProperty>::new(),
        child_nodes: Vec::<Box<DtbNode>>::new(),
    };

    root.init(buffer);

    // sdcard patch
    
    match root.find_childnode("sdhci@3440000") {
        Some(sdhci) => {
            let status = sdhci.find_property("status").unwrap();
            status.value = Some(String::from("\"okay\""));
        },
        None => {
            println!("Orin NX does not support microSD");
        }
    }

    // camera patch
    let cam_i2c0 = root.find_childnode("cam_i2cmux").unwrap()
                        .find_childnode("i2c@0").unwrap();

    let rbpcv3_imx477_a_1a = cam_i2c0.find_childnode("rbpcv3_imx477_a@1a").unwrap();

    rbpcv3_imx477_a_1a.find_childnode("mode0").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv3_imx477_a_1a.find_childnode("mode1").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv3_imx477_a_1a.find_childnode("ports").unwrap()
                    .find_childnode("port@0").unwrap()
                    .find_childnode("endpoint").unwrap()
                    .find_property("port-index").unwrap()
                    .value = Some("<0x00>".to_string());

    let rbpcv2_imx219_a_10 = cam_i2c0.find_childnode("rbpcv2_imx219_a@10").unwrap();

    rbpcv2_imx219_a_10.find_childnode("mode0").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv2_imx219_a_10.find_childnode("mode1").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv2_imx219_a_10.find_childnode("mode2").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv2_imx219_a_10.find_childnode("mode3").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv2_imx219_a_10.find_childnode("mode4").unwrap()
                    .find_property("tegra_sinterface").unwrap()
                    .value = Some("\"serial_a\"".to_string());

    rbpcv2_imx219_a_10.find_childnode("ports").unwrap()
                    .find_childnode("port@0").unwrap()
                    .find_childnode("endpoint").unwrap()
                    .find_property("port-index").unwrap()
                    .value = Some("<0x00>".to_string());

    // apply root to new dts file
    
    let patched = root.stringify(0);
    let mut patched_dts = OpenOptions::new()
                                .write(true)
                                .truncate(true)
                                .create(true)
                                .open(&new_dts_filename)
                                .expect("Error : Cannot create new dts file");

    patched_dts.write_all(patched.as_bytes()).expect("Error : Cannot write to new dts file");

    // println!("{root:?}");
    
    // compile
    print!("Compile patched dts file... ");
    let _compile = Command::new("dtc")
                        .args(["-I", "dts", "-O", "dtb", &new_dts_filename, "-o", &new_dtb_filename])
                        .output()
                        .expect("Error : Failed to compiile patched dts");
    println!("OK");

    // if compile succeeded, add new boot menu to extlinux.conf
    let mut extlinux_file = OpenOptions::new()
                                        .write(true)
                                        .read(true)
                                        .open("/boot/extlinux/extlinux.conf")
                                        .expect("Error : Cannot open extlinux.conf");

    let mut extlinux_content = String::new();

    let default_entry = extlinux.entries.iter().find(|entry| entry.label.clone().unwrap() == extlinux.default.clone().unwrap()).unwrap();


    extlinux_content.push_str("\n\n");
    extlinux_content.push_str("LABEL patched_");
    extlinux_content.push_str(&default_entry.label.clone().unwrap());
    extlinux_content.push_str("\n\tMENU LABEL patched_");
    extlinux_content.push_str(&default_entry.menu_label.clone().unwrap());
    extlinux_content.push_str("\n\tLINUX ");
    extlinux_content.push_str(&default_entry.linux.clone().unwrap());
    extlinux_content.push_str("\n\tFDT ");
    extlinux_content.push_str(&new_dtb_filename);
    extlinux_content.push_str("\n\tINITRD ");
    extlinux_content.push_str(&default_entry.initrd.clone().unwrap());
    extlinux_content.push_str("\n\tAPPEND ");
    extlinux_content.push_str(&default_entry.append.clone().unwrap());
    extlinux_content.push_str("\n");

    extlinux_file.write_all(extlinux_content.as_bytes()).expect("Error : Cannot write to extlinux.conf");

    println!("Patch finished succesfully");
}
