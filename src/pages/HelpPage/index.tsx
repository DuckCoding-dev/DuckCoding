import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { BookOpen, Github, MessageCircle, FileText, HelpCircle } from 'lucide-react';
import { open } from '@tauri-apps/plugin-shell';

interface HelpPageProps {
  onShowOnboarding: () => void;
}

export function HelpPage({ onShowOnboarding }: HelpPageProps) {
  return (
    <div className="container mx-auto p-6 max-w-4xl">
      <div className="mb-6">
        <h1 className="text-3xl font-bold mb-2">帮助中心</h1>
        <p className="text-muted-foreground">获取使用帮助、查看文档或反馈问题</p>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        {/* 新手引导 */}
        <Card className="hover:shadow-lg transition-shadow">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BookOpen className="h-5 w-5 text-primary" />
              新手引导
            </CardTitle>
            <CardDescription>重新查看应用的基础配置和功能介绍</CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={onShowOnboarding} className="w-full">
              <BookOpen className="mr-2 h-4 w-4" />
              查看新手引导
            </Button>
          </CardContent>
        </Card>

        {/* 使用文档 */}
        <Card className="hover:shadow-lg transition-shadow">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FileText className="h-5 w-5 text-primary" />
              使用文档
            </CardTitle>
            <CardDescription>查看详细的功能说明和使用指南</CardDescription>
          </CardHeader>
          <CardContent>
            <Button
              variant="outline"
              className="w-full"
              onClick={() =>
                open('https://github.com/DuckCoding-dev/DuckCoding/blob/main/README.md')
              }
            >
              <FileText className="mr-2 h-4 w-4" />
              阅读文档
            </Button>
          </CardContent>
        </Card>

        {/* 问题反馈 */}
        <Card className="hover:shadow-lg transition-shadow">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <MessageCircle className="h-5 w-5 text-primary" />
              问题反馈
            </CardTitle>
            <CardDescription>遇到问题？向我们反馈 Bug 或提出建议</CardDescription>
          </CardHeader>
          <CardContent>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => open('https://github.com/DuckCoding-dev/DuckCoding/issues')}
            >
              <MessageCircle className="mr-2 h-4 w-4" />
              提交问题
            </Button>
          </CardContent>
        </Card>

        {/* GitHub 仓库 */}
        <Card className="hover:shadow-lg transition-shadow">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Github className="h-5 w-5 text-primary" />
              GitHub 仓库
            </CardTitle>
            <CardDescription>查看源代码、参与贡献或 Star 项目</CardDescription>
          </CardHeader>
          <CardContent>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => open('https://github.com/DuckCoding-dev/DuckCoding')}
            >
              <Github className="mr-2 h-4 w-4" />
              访问仓库
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* 常见问题 */}
      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HelpCircle className="h-5 w-5 text-primary" />
            常见问题
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="border-l-4 border-primary pl-4">
            <h3 className="font-semibold mb-1">如何配置工具的 API Key？</h3>
            <p className="text-sm text-muted-foreground">
              前往「配置 API」页面，为每个工具单独配置 API Key 和端点地址。
            </p>
          </div>

          <div className="border-l-4 border-primary pl-4">
            <h3 className="font-semibold mb-1">什么是透明代理？</h3>
            <p className="text-sm text-muted-foreground">
              透明代理允许您在不修改工具配置的情况下切换不同的 API 端点和密钥，支持会话级配置管理。
            </p>
          </div>

          <div className="border-l-4 border-primary pl-4">
            <h3 className="font-semibold mb-1">如何查看工具使用统计？</h3>
            <p className="text-sm text-muted-foreground">
              在「基本设置」中配置用户 ID 和系统令牌后，即可在「用量统计」页面查看详细的使用数据。
            </p>
          </div>

          <div className="border-l-4 border-primary pl-4">
            <h3 className="font-semibold mb-1">遇到网络问题怎么办？</h3>
            <p className="text-sm text-muted-foreground">
              可以在「设置 → 代理设置」中配置全局代理，支持 HTTP、HTTPS 和 SOCKS5 代理。
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
