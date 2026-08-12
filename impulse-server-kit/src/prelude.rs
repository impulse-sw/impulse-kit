//! Standard prelude to import needed tools at once.

pub use impulse_utils::{self, prelude::*};

pub use crate::security_headers::{SecurityHeaders, SecurityHeadersOptions};
pub use crate::seo::{
  ChangeFreq, MAX_SITEMAP_URLS, ROBOTS_TXT_PATH, RobotsGroup, RobotsTag, RobotsTxt, RobotsTxtHandler, SITEMAP_XML_PATH,
  Sitemap, SitemapHandler, SitemapUrl, request_origin, set_x_robots_tag,
};
pub use crate::setup::{GenericSetup, GenericValues, load_generic_config, load_generic_state};
pub use crate::startup::{get_root_router, get_root_router_autoinject, start};
pub use salvo;
pub use tracing;
pub use tracing::instrument;

pub use salvo::handler;
pub use salvo::{Depot, Request, Response, Router};

#[cfg(feature = "endpoint")]
pub use crate::endpoint::{Anonymous, IdentityResolver, endpoint_router};

#[cfg(feature = "oapi")]
pub use salvo::oapi::endpoint;

#[cfg(feature = "otel")]
pub use crate::otel;

#[cfg(feature = "telemetry")]
pub use crate::telemetry::{
  TelemetryAttr, TelemetryBatch, TelemetryEvent, TelemetryEventKind, TelemetryLevel, TelemetryRequestCtx,
  TelemetrySink, TracingTelemetrySink, collect_telemetry, default_telemetry_router, telemetry_router,
};

#[cfg(feature = "test")]
pub use crate::test_exts::*;

#[cfg(feature = "static-server")]
pub use crate::static_server::{
  CacheMap, NoRedirectStaticRouter, ProvidedRoutesStaticRouter, StaticRouter, assets_only_router_from, frontend_router,
  frontend_router_from_given_dist,
};

#[cfg(feature = "leptos-ssr")]
pub use crate::leptos_ssr::{
  CONTAINER_FRONTEND_DISTRIBUTABLE, FRONTEND_DIST_ENV, LOCAL_FRONTEND_DISTRIBUTABLE, LeptosOptions, PKG_SUBDIR,
  SeoDefaults, SsrStreamMode, assets_only_router, leptos_router,
};

#[cfg(feature = "leptos-server-fn")]
pub use crate::leptos_ssr::{ServerFnSalvoHandler, server_fn_router};

#[cfg(feature = "impulse-ring")]
pub use crate::impulse_ring::{ImpulseRingListener, serve_impulse_ring};
