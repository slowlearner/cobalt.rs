use std::fmt;
use std::path;

use liquid;
use serde_yaml;

use error::*;

use super::collection;
use super::files;
use super::frontmatter;
use super::assets;
use super::sass;
use super::site;

arg_enum! {
    #[derive(Serialize, Deserialize)]
    #[derive(Debug, PartialEq, Copy, Clone)]
    pub enum Dump {
        DocObject,
        DocTemplate,
        DocLinkObject,
        Document
    }
}

impl Dump {
    pub fn is_doc(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SyntaxHighlight {
    pub theme: String,
}

impl Default for SyntaxHighlight {
    fn default() -> Self {
        Self { theme: "base16-ocean.dark".to_owned() }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PageConfig {
    pub default: frontmatter::FrontmatterBuilder,
}

impl From<PageConfig> for collection::CollectionBuilder {
    fn from(config: PageConfig) -> Self {
        // Pages aren't publicly exposed as a collection
        let slug = Some("".to_owned());
        let dir = Some(".".to_owned());
        let default = config.default.merge_excerpt_separator("".to_owned());
        collection::CollectionBuilder {
            slug,
            dir,
            default: default,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PostConfig {
    pub title: Option<String>,
    pub description: Option<String>,
    pub dir: Option<String>,
    pub drafts_dir: Option<String>,
    pub order: collection::SortOrder,
    pub rss: Option<String>,
    pub jsonfeed: Option<String>,
    pub default: frontmatter::FrontmatterBuilder,
}

impl Default for PostConfig {
    fn default() -> Self {
        Self {
            title: Default::default(),
            description: Default::default(),
            dir: Some("posts".to_owned()),
            drafts_dir: Default::default(),
            order: Default::default(),
            rss: Default::default(),
            jsonfeed: Default::default(),
            default: Default::default(),
        }
    }
}

impl From<PostConfig> for collection::CollectionBuilder {
    fn from(config: PostConfig) -> Self {
        let PostConfig {
            title,
            description,
            dir,
            drafts_dir,
            order,
            rss,
            jsonfeed,
            default,
        } = config;

        let slug = Some("posts".to_owned());
        collection::CollectionBuilder {
            title,
            slug,
            description,
            dir,
            drafts_dir,
            order,
            rss,
            jsonfeed,
            default,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SiteConfig {
    pub title: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
    pub data: Option<liquid::Object>,
}

#[derive(Debug, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SassConfig {
    pub style: sass::SassOutputStyle,
}

#[derive(Debug, Clone, PartialEq, Default)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AssetsConfig {
    pub sass: SassConfig,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ConfigBuilder {
    #[serde(skip)]
    pub root: path::PathBuf,
    pub source: String,
    pub destination: String,
    #[serde(skip)]
    pub abs_dest: Option<String>,
    pub include_drafts: bool,
    pub default: frontmatter::FrontmatterBuilder,
    pub pages: PageConfig,
    pub posts: PostConfig,
    pub site: SiteConfig,
    pub template_extensions: Vec<String>,
    pub ignore: Vec<String>,
    pub syntax_highlight: SyntaxHighlight,
    #[serde(skip)]
    pub layouts_dir: &'static str,
    #[serde(skip)]
    pub includes_dir: &'static str,
    pub assets: AssetsConfig,
    // This is a debug-only field and should be transient rather than persistently set.
    #[serde(skip)]
    pub dump: Vec<Dump>,
}

impl Default for ConfigBuilder {
    fn default() -> ConfigBuilder {
        ConfigBuilder {
            root: Default::default(),
            source: "./".to_owned(),
            destination: "./_site".to_owned(),
            abs_dest: Default::default(),
            include_drafts: false,
            default: Default::default(),
            pages: Default::default(),
            posts: Default::default(),
            site: Default::default(),
            template_extensions: vec!["md".to_owned(), "liquid".to_owned()],
            ignore: Default::default(),
            syntax_highlight: SyntaxHighlight::default(),
            layouts_dir: "_layouts",
            includes_dir: "_includes",
            assets: AssetsConfig::default(),
            dump: Default::default(),
        }
    }
}

impl ConfigBuilder {
    pub fn from_file<P: Into<path::PathBuf>>(path: P) -> Result<ConfigBuilder> {
        Self::from_file_internal(path.into())
    }

    fn from_file_internal(path: path::PathBuf) -> Result<ConfigBuilder> {
        let content = files::read_file(&path)?;

        let mut config = if content.trim().is_empty() {
            ConfigBuilder::default()
        } else {
            serde_yaml::from_str(&content)?
        };

        let mut root = path;
        root.pop(); // Remove filename
        config.root = root;

        Ok(config)
    }

    pub fn from_cwd<P: Into<path::PathBuf>>(cwd: P) -> Result<ConfigBuilder> {
        Self::from_cwd_internal(cwd.into())
    }

    fn from_cwd_internal(cwd: path::PathBuf) -> Result<ConfigBuilder> {
        let file_path = files::find_project_file(&cwd, "_cobalt.yml");
        let config = file_path
            .map(|p| {
                     debug!("Using config file {:?}", &p);
                     Self::from_file(&p).chain_err(|| format!("Error reading config file {:?}", p))
                 })
            .unwrap_or_else(|| {
                warn!("No _cobalt.yml file found in current directory, using default config.");
                let config = ConfigBuilder {
                    root: cwd,
                    ..Default::default()
                };
                Ok(config)
            })?;
        Ok(config)
    }

    pub fn build(self) -> Result<Config> {
        let ConfigBuilder {
            root,
            source,
            destination,
            abs_dest,
            include_drafts,
            default,
            pages,
            posts,
            site,
            template_extensions,
            ignore,
            syntax_highlight,
            layouts_dir,
            includes_dir,
            assets,
            dump,
        } = self;

        if include_drafts {
            debug!("Draft mode enabled");
        }

        let source = files::cleanup_path(source);
        let destination = files::cleanup_path(destination);

        let mut ignore = ignore;
        if let Ok(rel_dest) = path::Path::new(&destination).strip_prefix(&source) {
            let rel_dest = rel_dest.to_str().expect("started as a utf-8 string");
            if !rel_dest.is_empty() {
                ignore.push(format!("/{}", rel_dest.to_owned()));
            }
        }

        let source = root.join(source);
        let destination = abs_dest
            .map(|s| s.into())
            .unwrap_or_else(|| root.join(destination));

        let site = site::SiteBuilder {
            title: site.title,
            description: site.description,
            base_url: site.base_url,
            data: site.data,
            ..Default::default()
        };
        let site = site.build(&source)?;

        let pages: collection::CollectionBuilder = pages.into();
        let mut pages = pages.merge_frontmatter(default.clone());
        // Use `site` because the pages are effectively the site
        pages.title = Some(site.title
                               .clone()
                               .unwrap_or_else(|| "".to_owned())
                               .to_owned());
        pages.description = site.description.clone();
        pages.include_drafts = false;
        pages.template_extensions = template_extensions.clone();
        pages.ignore = ignore.clone();
        let pages = pages.build()?;

        let posts: collection::CollectionBuilder = posts.into();
        let mut posts = posts.merge_frontmatter(default);
        // Default with `site` for people quickly bootstrapping a blog, the blog and site are
        // effectively equivalent.
        if posts.title.is_none() {
            posts.title = Some(site.title
                                   .clone()
                                   .unwrap_or_else(|| "".to_owned())
                                   .to_owned());
        }
        if posts.description.is_none() {
            posts.description = site.description.clone();
        }
        posts.include_drafts = include_drafts;
        posts.template_extensions = template_extensions.clone();
        posts.ignore = ignore.clone();
        let posts = posts.build()?;

        let assets = {
            let mut sass = sass::SassBuilder::new();
            sass.style = assets.sass.style;
            let mut assets = assets::AssetsBuilder::default();
            assets.sass = sass;
            assets.source = Some(source.clone());
            assets.ignore = ignore.clone();
            assets.template_extensions = template_extensions.clone();
            assets
        };
        let assets = assets.build()?;

        let config = Config {
            source,
            destination,
            pages,
            posts,
            site,
            ignore,
            syntax_highlight,
            layouts_dir,
            includes_dir,
            assets,
            dump,
        };

        Ok(config)
    }
}

impl fmt::Display for ConfigBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut converted = serde_yaml::to_string(self).map_err(|_| fmt::Error)?;
        converted.drain(..4);
        write!(f, "{}", converted)
    }
}

#[derive(Debug)]
pub struct Config {
    pub source: path::PathBuf,
    pub destination: path::PathBuf,
    pub pages: collection::Collection,
    pub posts: collection::Collection,
    pub site: site::Site,
    pub ignore: Vec<String>, // HACK: Here until migrate doesn't need it
    pub syntax_highlight: SyntaxHighlight,
    pub layouts_dir: &'static str,
    pub includes_dir: &'static str,
    pub assets: assets::Assets,
    pub dump: Vec<Dump>,
}

impl Default for Config {
    fn default() -> Config {
        ConfigBuilder::default()
            .build()
            .expect("default config should not fail")
    }
}

#[test]
fn test_from_file_ok() {
    let result = ConfigBuilder::from_file("tests/fixtures/config/_cobalt.yml").unwrap();
    assert_eq!(result.root,
               path::Path::new("tests/fixtures/config").to_path_buf());
}

#[test]
fn test_from_file_alternate_name() {
    let result = ConfigBuilder::from_file("tests/fixtures/config/rss.yml").unwrap();
    assert_eq!(result.root,
               path::Path::new("tests/fixtures/config").to_path_buf());
}

#[test]
fn test_from_file_empty() {
    let result = ConfigBuilder::from_file("tests/fixtures/config/empty.yml").unwrap();
    assert_eq!(result.root,
               path::Path::new("tests/fixtures/config").to_path_buf());
}

#[test]
fn test_from_file_invalid_syntax() {
    let result = ConfigBuilder::from_file("tests/fixtures/config/invalid_syntax.yml");
    assert!(result.is_err());
}

#[test]
fn test_from_file_not_found() {
    let result = ConfigBuilder::from_file("tests/fixtures/config/config_does_not_exist.yml");
    assert!(result.is_err());
}

#[test]
fn test_from_cwd_ok() {
    let result = ConfigBuilder::from_cwd("tests/fixtures/config/child").unwrap();
    assert_eq!(result.root,
               path::Path::new("tests/fixtures/config").to_path_buf());
}

#[test]
fn test_from_cwd_not_found() {
    let result = ConfigBuilder::from_cwd("tests/fixtures").unwrap();
    assert_eq!(result.root, path::Path::new("tests/fixtures").to_path_buf());
}

#[test]
fn test_build_default() {
    let config = ConfigBuilder::default();
    config.build().unwrap();
}

#[test]
fn test_build_dest() {
    let result = ConfigBuilder::from_file("tests/fixtures/config/_cobalt.yml").unwrap();
    let result = result.build().unwrap();
    assert_eq!(result.source,
               path::Path::new("tests/fixtures/config").to_path_buf());
    assert_eq!(result.destination,
               path::Path::new("tests/fixtures/config/dest").to_path_buf());
}

#[test]
fn test_build_abs_dest() {
    let mut result = ConfigBuilder::from_file("tests/fixtures/config/_cobalt.yml").unwrap();
    result.abs_dest = Some("hello/world".to_owned());
    let result = result.build().unwrap();
    assert_eq!(result.source,
               path::Path::new("tests/fixtures/config").to_path_buf());
    assert_eq!(result.destination,
               path::Path::new("hello/world").to_path_buf());
}

#[test]
fn test_build_posts_rel() {
    let mut config = ConfigBuilder::default();
    config.posts.dir = Some("rel".to_owned());
    let config = config.build().unwrap();
    assert_eq!(config.posts.dir, "rel");
}

#[test]
fn test_build_posts_abs() {
    let mut config = ConfigBuilder::default();
    config.posts.dir = Some("/root".to_owned());
    assert!(config.build().is_err());
}

#[test]
fn test_build_drafts_rel() {
    let mut config = ConfigBuilder::default();
    config.posts.drafts_dir = Some("rel".into());
    let config = config.build().unwrap();
    assert_eq!(config.posts.drafts_dir, Some("rel".into()));
}

#[test]
fn test_build_drafts_abs() {
    let mut config = ConfigBuilder::default();
    config.posts.drafts_dir = Some("/root".into());
    assert!(config.build().is_err());
}
