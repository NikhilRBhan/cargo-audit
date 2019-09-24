//! Presenter for `rustsec::Report` information.

use crate::{
    config::{OutputConfig, OutputFormat},
    prelude::*,
};
use abscissa_core::terminal::{
    self,
    Color::{self, Red, Yellow},
};
use rustsec::{
    cargo_lock::{
        dependency::{self, graph::EdgeDirection, Dependency},
        Lockfile, Package,
    },
    Vulnerability, Warning,
};
use std::{collections::BTreeSet as Set, io, path::Path};

/// Vulnerability information presenter
#[derive(Clone, Debug)]
pub struct Presenter {
    /// Track packages we've displayed once so we don't show the same dep tree
    // TODO(tarcieri): group advisories about the same package?
    displayed_packages: Set<Dependency>,

    /// Output configuration
    config: OutputConfig,
}

impl Presenter {
    /// Create a new vulnerability information presenter
    pub fn new(config: &OutputConfig) -> Self {
        Self {
            displayed_packages: Set::new(),
            config: config.clone(),
        }
    }

    /// Information to display before a report is generated
    pub fn before_report(&mut self, lockfile_path: &Path, lockfile: &Lockfile) {
        if !self.config.is_quiet() {
            status_ok!(
                "Scanning",
                "{} for vulnerabilities ({} crate dependencies)",
                lockfile_path.display(),
                lockfile.packages.len(),
            );
        }
    }

    /// Print the vulnerability report generated by an audit
    pub fn print_report(&mut self, report: &rustsec::Report, lockfile: &Lockfile) {
        if self.config.format == OutputFormat::Json {
            serde_json::to_writer(io::stdout(), &report).unwrap();
            return;
        }

        if report.vulnerabilities.found {
            status_err!("Vulnerable crates found!");
        } else {
            status_ok!("Success", "No vulnerable packages found");
        }

        let tree = lockfile
            .dependency_tree()
            .expect("invalid Cargo.lock dependency tree");

        for vulnerability in &report.vulnerabilities.list {
            self.print_vulnerability(vulnerability, &tree);
        }

        if !report.warnings.is_empty() {
            println!();
            status_warn!("found informational advisories for dependencies");

            for warning in &report.warnings {
                self.print_warning(warning, &tree)
            }
        }

        if report.vulnerabilities.found {
            println!();

            if report.vulnerabilities.count == 1 {
                status_err!("1 vulnerability found!");
            } else {
                status_err!("{} vulnerabilities found!", report.vulnerabilities.count);
            }
        }
    }

    /// Print information about the given vulnerability
    fn print_vulnerability(&mut self, vulnerability: &Vulnerability, tree: &dependency::Tree) {
        let advisory = &vulnerability.advisory;

        println!();
        self.print_attr(Red, "ID:      ", &advisory.id);
        self.print_attr(Red, "Crate:   ", &vulnerability.package.name);
        self.print_attr(Red, "Version: ", &vulnerability.package.version.to_string());
        self.print_attr(Red, "Date:    ", &advisory.date);

        if let Some(url) = advisory.id.url() {
            self.print_attr(Red, "URL:     ", &url);
        } else if let Some(url) = &advisory.url {
            self.print_attr(Red, "URL:     ", url);
        }

        self.print_attr(Red, "Title:   ", &advisory.title);
        self.print_attr(
            Red,
            "Solution: upgrade to",
            &vulnerability
                .versions
                .patched
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .as_slice()
                .join(" OR "),
        );

        self.print_tree(Red, &vulnerability.package, tree);
    }

    /// Print information about a given warning
    fn print_warning(&mut self, warning: &Warning, tree: &dependency::Tree) {
        println!();

        self.print_attr(Yellow, "Crate:   ", &warning.package.name);
        self.print_attr(Red, "Title: ", &warning.advisory.title);
        self.print_attr(Red, "Date:    ", &warning.advisory.date);

        if let Some(url) = warning.advisory.id.url() {
            self.print_attr(Yellow, "URL:     ", &url);
        } else if let Some(url) = &warning.advisory.url {
            self.print_attr(Yellow, "URL:     ", url);
        }

        self.print_tree(Yellow, &warning.package, tree);
    }

    /// Display an attribute of a particular vulnerability
    fn print_attr(&self, color: Color, attr: &str, content: impl AsRef<str>) {
        terminal::status::Status::new()
            .bold()
            .color(color)
            .status(attr)
            .print_stdout(content.as_ref())
            .unwrap();
    }

    /// Print the inverse dependency tree to standard output
    fn print_tree(&mut self, color: Color, package: &Package, tree: &dependency::Tree) {
        // Only show the tree once per package
        if !self
            .displayed_packages
            .insert(Dependency::from(package.clone()))
        {
            return;
        }

        if !self.config.show_tree.unwrap_or(true) {
            return;
        }

        terminal::status::Status::new()
            .bold()
            .color(color)
            .status("Dependency tree:")
            .print_stdout("")
            .unwrap();

        let package_node = tree.nodes()[&Dependency::from(package.clone())];
        tree.render(&mut io::stdout(), package_node, EdgeDirection::Incoming)
            .unwrap();
    }
}
