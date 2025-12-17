//! HTTP Body 类型定义
//!
//! 统一的 BoxBody 实现，供 proxy_instance 使用

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Body, Frame};
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

pin_project! {
    pub struct BoxBody {
        #[pin]
        inner: Pin<Box<dyn Body<Data = Bytes, Error = Box<dyn std::error::Error + Send + Sync>> + Send>>,
    }
}

impl Body for BoxBody {
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().inner.poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        self.inner.size_hint()
    }
}

/// 创建 BoxBody 的辅助函数
pub fn box_body<B>(body: B) -> BoxBody
where
    B: Body<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    BoxBody {
        inner: Box::pin(body.map_err(Into::into)),
    }
}
