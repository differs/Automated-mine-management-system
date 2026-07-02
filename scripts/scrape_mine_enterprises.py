#!/usr/bin/env python3
"""
自动从三大免费数据源抓取矿山企业名单：
  1. 各省自然资源厅（采矿权/矿业权/绿色矿山公示）
  2. 国家矿山安全监察局（煤矿标准化名单）
  3. 各省应急管理局（安全生产许可证企业名单）

使用方法：
    # 从所有数据源抓取
    python scripts/scrape_mine_enterprises.py

    # 只抓特定数据源
    python scripts/scrape_mine_enterprises.py --source natural-resources  # 自然资源厅
    python scripts/scrape_mine_enterprises.py --source safety            # 国家矿山安监局
    python scripts/scrape_mine_enterprises.py --source emergency         # 应急管理局
    python scripts/scrape_mine_enterprises.py --source all               # 同上

    # 只抓特定省份
    python scripts/scrape_mine_enterprises.py --provinces 山西 陕西

    # 只抓近30天
    python scripts/scrape_mine_enterprises.py --days 30

依赖：pip install httpx beautifulsoup4 lxml
"""

import sys
import csv
import re
import time
import random
from datetime import datetime, timedelta
from pathlib import Path
from urllib.parse import urljoin, urlparse
from dataclasses import dataclass, field, asdict

import httpx
from bs4 import BeautifulSoup

# ─── 配置 ──────────────────────────────────────────────────────────────────

MIN_DELAY = 1.5
MAX_DELAY = 3.0
TIMEOUT = 30

DEFAULT_OUTPUT = Path(__file__).parent.parent / "docs" / "scraped_enterprises.csv"

# 关键词：页面标题包含这些词才进入详情抓取
MINING_KEYWORDS = [
    # 自然资源厅相关
    "绿色矿山", "采矿权", "矿业权", "矿山", "煤矿", "采矿", "勘查开采",
    # 国家矿山安监局相关
    "煤矿安全生产标准化", "一级企业名单", "标准化一级", "一级达标煤矿",
    "安全生产许可证", "安全许可",
    # 应急管理局相关
    "非煤矿山", "非煤矿",
]


# ─── 数据结构 ──────────────────────────────────────────────────────────────

@dataclass
class Enterprise:
    name: str
    province: str = ""
    source_url: str = ""
    source_title: str = ""
    source_date: str = ""
    mine_type: str = ""
    contact_phone: str = ""
    website: str = ""
    address: str = ""
    notes: str = ""


@dataclass
class ScraperConfig:
    name: str
    list_url: str
    list_selector: str
    title_selector: str = "a"
    date_selector: str = "span"
    link_prefix: str = ""
    pagination_pattern: str = ""
    total_pages: int = 5
    charset: str = "utf-8"
    source_type: str = "natural-resources"  # natural-resources | safety | emergency


# ─── 三大数据源配置 ─────────────────────────────────────────────────────

# 1) 各省自然资源厅
NATURAL_RESOURCES_CONFIGS = [
    ScraperConfig(name="山西", list_url="https://zrzyt.shanxi.gov.cn/zwgk/tzgg/",
        list_selector="ul.zwgk_right_content li", link_prefix="https://zrzyt.shanxi.gov.cn/zwgk/tzgg/",
        pagination_pattern="https://zrzyt.shanxi.gov.cn/zwgk/tzgg/index_{}.shtml"),
    ScraperConfig(name="陕西", list_url="https://zrzyt.shaanxi.gov.cn/news/tzgg/",
        list_selector="ul.gl-news-list li", link_prefix="https://zrzyt.shaanxi.gov.cn/news/tzgg/",
        pagination_pattern="https://zrzyt.shaanxi.gov.cn/news/tzgg/index_{}.html"),
    ScraperConfig(name="贵州", list_url="https://zrzy.guizhou.gov.cn/wzgb/xwzx/tzgg/",
        list_selector="ul.NewsList li", title_selector="a h1",
        date_selector="div.listItem_infor span",
        link_prefix="https://zrzy.guizhou.gov.cn/wzgb/xwzx/tzgg/",
        pagination_pattern="https://zrzy.guizhou.gov.cn/wzgb/xwzx/tzgg/index_{}.html"),
    ScraperConfig(name="山东", list_url="http://dnr.shandong.gov.cn/zwgk_324/gs/",
        list_selector="ul.news-list li", link_prefix="http://dnr.shandong.gov.cn/",
        pagination_pattern="http://dnr.shandong.gov.cn/zwgk_324/gs/index_{}.html"),
    ScraperConfig(name="安徽", list_url="https://zrzyt.ah.gov.cn/xwdt/tzgg/",
        list_selector="ul.news_list li", link_prefix="https://zrzyt.ah.gov.cn",
        pagination_pattern="https://zrzyt.ah.gov.cn/xwdt/tzgg/index_{}.html"),
    ScraperConfig(name="云南", list_url="https://dnr.yn.gov.cn/html/zhengwugongkai/tongzhigonggao/",
        list_selector="ul.list-item li", link_prefix="https://dnr.yn.gov.cn",
        pagination_pattern="https://dnr.yn.gov.cn/html/zhengwugongkai/tongzhigonggao/index_{}.html"),
    ScraperConfig(name="四川", list_url="http://dnr.sc.gov.cn/scdnr/scgsgg/",
        list_selector="ul.news-list li", link_prefix="http://dnr.sc.gov.cn",
        pagination_pattern="http://dnr.sc.gov.cn/scdnr/scgsgg/index_{}.html"),
    ScraperConfig(name="河南", list_url="https://dnr.henan.gov.cn/gk/gsgg/lsksml/",
        list_selector="ul.news-list li", link_prefix="https://dnr.henan.gov.cn",
        pagination_pattern="https://dnr.henan.gov.cn/gk/gsgg/lsksml/index_{}.html"),
    ScraperConfig(name="甘肃", list_url="http://zrzy.gansu.gov.cn/zrzy/c107676/",
        list_selector="ul.news-list li", link_prefix="http://zrzy.gansu.gov.cn",
        pagination_pattern="http://zrzy.gansu.gov.cn/zrzy/c107676/index_{}.html"),
    ScraperConfig(name="内蒙古", list_url="https://zrzy.nmg.gov.cn/zwgk/gsgg/qtgsgg/",
        list_selector="ul.news-list li", link_prefix="https://zrzy.nmg.gov.cn",
        pagination_pattern="https://zrzy.nmg.gov.cn/zwgk/gsgg/qtgsgg/index_{}.html"),
    ScraperConfig(name="河北", list_url="https://zrzy.hebei.gov.cn/heb/gongk/gkml/gggs/",
        list_selector="ul.news-list li", link_prefix="https://zrzy.hebei.gov.cn",
        pagination_pattern="https://zrzy.hebei.gov.cn/heb/gongk/gkml/gggs/index_{}.html",
        total_pages=3),
    ScraperConfig(name="新疆", list_url="https://zrzyt.xinjiang.gov.cn/xjgtzy/tzgg/",
        list_selector="ul.list-list li", link_prefix="https://zrzyt.xinjiang.gov.cn",
        pagination_pattern="https://zrzyt.xinjiang.gov.cn/xjgtzy/tzgg/index_{}.html"),
    ScraperConfig(name="湖南", list_url="http://zrzyt.hunan.gov.cn/xxgk/tzgg/index.html",
        list_selector="ul.news-list li", link_prefix="http://zrzyt.hunan.gov.cn",
        pagination_pattern="http://zrzyt.hunan.gov.cn/xxgk/tzgg/index_{}.html",
        total_pages=3),
    ScraperConfig(name="广东", list_url="https://nr.gd.gov.cn/zwgknew/tzgg/gg/",
        list_selector="ul.news-list li", link_prefix="https://nr.gd.gov.cn",
        pagination_pattern="https://nr.gd.gov.cn/zwgknew/tzgg/gg/index_{}.html",
        total_pages=3),
]

# 2) 国家矿山安全监察局
SAFETY_CONFIGS = [
    ScraperConfig(name="国家矿山安监局-通知公告",
        list_url="https://www.chinamine-safety.gov.cn/zfxxgk/fdzdgknr/tzgg/index.shtml",
        list_selector="#ogi-list li",
        title_selector="a",
        date_selector="span",
        link_prefix="https://www.chinamine-safety.gov.cn/zfxxgk/fdzdgknr/tzgg/",
        pagination_pattern="https://www.chinamine-safety.gov.cn/zfxxgk/fdzdgknr/tzgg/index_{}.shtml",
        total_pages=5,
        source_type="safety"),
    ScraperConfig(name="国家矿山安监局-安全许可",
        list_url="https://www.chinamine-safety.gov.cn/zfxxgk/fdzdgknr/anqxk/",
        list_selector="#ogi-list li",
        title_selector="a",
        date_selector="span",
        link_prefix="https://www.chinamine-safety.gov.cn/zfxxgk/fdzdgknr/anqxk/",
        pagination_pattern="https://www.chinamine-safety.gov.cn/zfxxgk/fdzdgknr/anqxk/index_{}.shtml",
        total_pages=5,
        source_type="safety"),
]

# 3) 各省应急管理局（安全生产许可证名单）
EMERGENCY_CONFIGS = [
    ScraperConfig(name="陕西应急厅-公告公示(2026)",
        list_url="https://yjt.shaanxi.gov.cn/gk/zcwj/wjzl/gggs/gg2026/",
        list_selector="ul.cm-news-list li",
        title_selector="a",
        date_selector="span.con-times",
        link_prefix="https://yjt.shaanxi.gov.cn",
        pagination_pattern="https://yjt.shaanxi.gov.cn/gk/zcwj/wjzl/gggs/gg2026/index_{}.html",
        total_pages=3,
        source_type="emergency"),
    ScraperConfig(name="陕西应急厅-公告公示(2025)",
        list_url="https://yjt.shaanxi.gov.cn/gk/zcwj/wjzl/gggs/gg2025/",
        list_selector="ul.cm-news-list li",
        title_selector="a",
        date_selector="span.con-times",
        link_prefix="https://yjt.shaanxi.gov.cn",
        pagination_pattern="https://yjt.shaanxi.gov.cn/gk/zcwj/wjzl/gggs/gg2025/index_{}.html",
        total_pages=5,
        source_type="emergency"),
]


# ─── 工具函数 ──────────────────────────────────────────────────────────────

def is_mining_related(title: str) -> bool:
    title_lower = title.lower()
    return any(kw in title or kw in title_lower for kw in MINING_KEYWORDS)


def extract_company_names(text: str) -> list[str]:
    """从文本中提取企业/矿山名称（三阶段提取）"""
    names = []

    # 阶段1: 完整有限公司/集团名
    for pat in [
        r"([\u4e00-\u9fff（）\(\)\w]{2,30}(?:有限公司|有限责任公司|股份有限公司))",
        r"([\u4e00-\u9fff（）\(\)\w]{2,20}(?:集团公司|集团))",
    ]:
        for m in re.finditer(pat, text):
            name = m.group(1).strip()
            if is_valid_company_name(name):
                names.append(name)

    # 阶段2: 矿山名
    for pat in [
        r"([\u4e00-\u9fff（）\(\)\w]{2,30}(?:煤矿|金矿|铁矿|铜矿|铝土矿|铅锌矿|镍矿|钨矿|锡矿|钼矿|锑矿|磷矿|萤石矿|石灰石矿|石灰岩矿|花岗岩矿|砂石矿|盐矿|石膏矿|重晶石矿|锰矿|钒矿))",
        r"([\u4e00-\u9fff（）\(\)\w]{2,20}?矿业[\u4e00-\u9fff（）\(\)\w]{1,10}?有限公司)",
    ]:
        for m in re.finditer(pat, text):
            name = m.group(1).strip()
            if is_valid_mine_name(name):
                names.append(name)

    # 阶段3: 从标题中的"关于...公示"结构提取
    for pat in [
        r"关于(?:将|对|同意|批准)?\s*[《「『]?\s*([\u4e00-\u9fff（）\(\)\w]{2,30}(?:有限公司|有限责任公司|股份有限公司|集团公司|集团|煤矿|金矿|铁矿|铜矿))",
        r"(?:将|对|认定)\s*([\u4e00-\u9fff（）\(\)\w]{2,40}?)\s*(?:纳入|列入|拟|通过|授予|颁发|出让|转让|公示|公告|注销|撤销|移出|为)",
    ]:
        for m in re.finditer(pat, text):
            name = m.group(1).strip()
            if is_valid_company_name(name) or is_valid_mine_name(name):
                names.append(name)

    seen = set()
    return [n for n in names if not (n in seen or seen.add(n))]


NON_COMPANY_PHRASES = {
    "资源采矿", "采矿权", "矿业权", "出让采矿", "申请办理采矿",
    "矿区生态修复", "生态修复方案", "矿山地质环境", "地质环境保护",
    "土地复垦", "开发利用方案", "开采方案", "矿产资源",
    "勘查开采", "勘查方案", "评审结果", "审查结果", "通过审查",
    "在公示期", "公示期内", "公示期间", "无异议", "如有异议",
    "现将", "现予以", "现予", "附件", "联系电话", "通讯地址",
    "陕西省矿", "省自然资源厅", "自然资源厅",
    "为做好", "竞买人", "本次采矿", "未处置",
    "业权人", "探矿权", "采矿许可证", "有效期限", "开采标高",
    "资源储量", "矿区范围", "拐点坐标", "开采矿种",
    "业权出让收益", "权转让给",
}


def is_valid_company_name(name: str) -> bool:
    name = name.strip()
    if len(name) < 4 or name in NON_COMPANY_PHRASES:
        return False
    if not any(name.endswith(s) for s in ["有限公司", "有限责任公司", "股份有限公司", "集团公司", "集团"]):
        return False
    bad_starts = ["关于", "现将", "对", "将", "权", "业", "矿", "资源", "我省", "本次"]
    if any(name.startswith(bs) for bs in bad_starts):
        return False
    bad_words = ["开采", "方案", "公示", "公告", "审查", "评审", "矿区", "收益", "出让", "转让", "评估", "报告"]
    if any(bw in name for bw in bad_words):
        return False
    bad_endings = ["开", "的", "与", "和", "及", "、"]
    if any(name.endswith(be) for be in bad_endings):
        return False
    return True


def is_valid_mine_name(name: str) -> bool:
    name = name.strip()
    if len(name) < 4 or name in NON_COMPANY_PHRASES:
        return False
    mine_suffixes = ["煤矿", "金矿", "铁矿", "铜矿", "铝土矿", "铅锌矿", "镍矿",
                     "钨矿", "锡矿", "钼矿", "锑矿", "磷矿", "萤石矿",
                     "石灰石矿", "石灰岩矿", "花岗岩矿", "砂石矿", "盐矿",
                     "石膏矿", "重晶石矿", "锰矿", "钒矿", "矿"]
    if not any(name.endswith(s) for s in mine_suffixes):
        return False
    bad_starts = ["关于", "现将", "对", "权", "业", "资源", "本次", "矿区", "生态", "我省", "采矿权"]
    if any(name.startswith(bs) for bs in bad_starts):
        return False
    if name in ("矿", "采矿", "矿业"):
        return False
    bad_words = ["开采", "方案", "公示", "公告", "审查", "评审", "评估报告", "收益", "出让"]
    if any(bw in name for bw in bad_words):
        return False
    bad_endings = ["开", "的", "与", "和", "及"]
    if any(name.endswith(be) for be in bad_endings):
        return False
    return True


def guess_mine_type(name: str) -> str:
    for kw, mt in {
        "煤": "煤炭", "金": "金矿", "铁": "铁矿", "铜": "铜矿",
        "铝": "铝土矿", "铅": "铅锌矿", "锌": "铅锌矿", "镍": "镍矿",
        "钨": "钨矿", "锡": "锡矿", "钼": "钼矿", "锑": "锑矿",
        "磷": "磷矿", "萤石": "萤石", "石灰石": "石灰岩", "石灰岩": "石灰岩",
        "花岗岩": "花岗岩", "砂石": "砂石", "水泥": "水泥用灰岩",
        "地热": "地热", "盐": "盐矿", "矿泉水": "矿泉水",
        "稀土": "稀土", "石油": "石油", "天然气": "天然气",
    }.items():
        if kw in name:
            return mt
    return "未知"


def fetch_page(client: httpx.Client, url: str, charset: str = "utf-8") -> str | None:
    for attempt in range(3):
        try:
            resp = client.get(url, timeout=httpx.Timeout(timeout=TIMEOUT, connect=15.0))
            resp.encoding = charset
            if resp.status_code == 200:
                return resp.text
            elif resp.status_code in (404, 403):
                return None
            else:
                print(f"  ⚠ HTTP {resp.status_code}")
        except httpx.TimeoutException:
            print(f"  ⚠ 超时(尝试{attempt+1}/3)")
        except Exception as e:
            print(f"  ⚠ 失败(尝试{attempt+1}/3): {type(e).__name__}")
            time.sleep(2)
    return None


def parse_list_page(html: str, config: ScraperConfig) -> list[dict]:
    soup = BeautifulSoup(html, "lxml")
    items = soup.select(config.list_selector)
    results = []

    for item in items:
        title_el = item.select_one(config.title_selector) if config.title_selector else item.find("a")
        date_el = item.select_one(config.date_selector) if config.date_selector else None
        if not title_el:
            continue

        # 提取标题（部分网站：<a>标题<span>日期</span></a>）
        title = title_el.get_text(strip=True)
        # 如果标题末尾有日期格式字符串，去掉
        title = re.sub(r'\d{4}-\d{2}-\d{2}$', '', title).strip()

        href = title_el.get("href", "")
        if not href and title_el.parent and title_el.parent.name == "a":
            href = title_el.parent.get("href", "")
        if not title or not href:
            continue

        date = date_el.get_text(strip=True) if date_el else ""
        date = re.sub(r'[\[\]\(\)（）]', '', date).strip()

        if href.startswith("http"):
            full_url = href
        elif href.startswith("/"):
            parsed = urlparse(config.list_url)
            full_url = f"{parsed.scheme}://{parsed.netloc}{href}"
        else:
            full_url = urljoin(config.link_prefix, href)

        results.append({"title": title, "url": full_url, "date": date})

    return results


def parse_detail_page(html: str) -> str:
    soup = BeautifulSoup(html, "lxml")
    for tag in soup(["script", "style", "nav", "footer", "header"]):
        tag.decompose()
    for sel in ["div.content", "div.article", "div.main-content", "div.text",
                 "div.TRS_Editor", "div.Custom_UnionStyle", "article",
                 "#content", "#UCAP-CONTENT", ".article-content", ".news-content"]:
        el = soup.select_one(sel)
        if el:
            return el.get_text(separator="\n", strip=True)
    return soup.get_text(separator="\n", strip=True)


def scrape_source(config: ScraperConfig, days: int = 0) -> list[Enterprise]:
    """抓取一个数据源的矿山企业"""
    source_label = {"natural-resources": "自然资源厅", "safety": "国家矿山安监局", "emergency": "应急管理局"}
    label = source_label.get(config.source_type, config.source_type)

    print(f"\n{'='*60}")
    print(f"📍 [{label}] {config.name}")
    print(f"{'='*60}")

    client = httpx.Client(headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                      "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
    }, follow_redirects=True, timeout=httpx.Timeout(timeout=TIMEOUT, connect=15.0),
       verify=False)

    cutoff = (datetime.now() - timedelta(days=days)) if days > 0 else None

    # ── 遍历列表页 ──
    all_entries = []
    seen_urls = set()
    for page in range(config.total_pages):
        url = config.list_url if page == 0 else config.pagination_pattern.format(page)
        print(f"  列表页 {page+1}/{config.total_pages}: {url}")
        html = fetch_page(client, url, config.charset)
        if not html:
            print(f"  → 停止翻页")
            break
        entries = parse_list_page(html, config)
        print(f"  → 找到 {len(entries)} 条")
        for e in entries:
            if e["url"] not in seen_urls:
                seen_urls.add(e["url"])
                all_entries.append(e)
        time.sleep(random.uniform(MIN_DELAY, MAX_DELAY))

    print(f"\n  共 {len(all_entries)} 条")

    # ── 筛选矿山相关 ──
    mining_entries = [e for e in all_entries if is_mining_related(e["title"])]
    print(f"  矿山相关: {len(mining_entries)}")

    # ── 访问详情提取企业名 ──
    enterprises = []
    for i, entry in enumerate(mining_entries):
        print(f"  详情 {i+1}/{len(mining_entries)}: {entry['title'][:50]}...", end=" ")
        html = fetch_page(client, entry["url"])
        if not html:
            print("❌")
            continue
        text = parse_detail_page(html)
        combined = entry["title"] + "\n" + text
        names = extract_company_names(combined)
        if names:
            for name in names[:5]:
                enterprises.append(Enterprise(
                    name=name,
                    province=config.name.replace("应急厅", "").replace("安监局", ""),
                    source_url=entry["url"],
                    source_title=entry["title"],
                    source_date=entry["date"],
                    mine_type=guess_mine_type(name),
                    notes=f"来源: {label}",
                ))
            print(f"✅ {len(names)}个: {names[:2]}")
        else:
            print("⏭️")
        time.sleep(random.uniform(MIN_DELAY, MAX_DELAY))

    # ── 去重 ──
    seen = set()
    unique = []
    for ent in enterprises:
        if ent.name not in seen:
            seen.add(ent.name)
            unique.append(ent)
    print(f"\n  ✅ 提取 {len(unique)} 家")
    return unique


# ─── I/O ────────────────────────────────────────────────────────────────────

def write_csv(enterprises: list[Enterprise], output_path: Path):
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", newline="", encoding="utf-8-sig") as f:
        w = csv.DictWriter(f, fieldnames=["province", "name", "mine_type", "source_date",
                                           "source_title", "source_url", "contact_phone",
                                           "website", "address", "notes"])
        w.writeheader()
        for e in enterprises:
            w.writerow(asdict(e))
    print(f"\n📁 {output_path}  ({len(enterprises)} 条)")


def print_summary(enterprises: list[Enterprise]):
    provinces, types = {}, {}
    for e in enterprises:
        provinces[e.province or "全国"] = provinces.get(e.province or "全国", 0) + 1
        t = e.mine_type if e.mine_type != "未知" else "其他"
        types[t] = types.get(t, 0) + 1
    print(f"\n{'='*60}\n📊 汇总\n{'='*60}")
    print("按省份/来源:"), print(*(f"  {p}: {c}家" for p, c in sorted(provinces.items(), key=lambda x: -x[1])), sep="\n")
    print("按矿种:"), print(*(f"  {t}: {c}家" for t, c in sorted(types.items(), key=lambda x: -x[1])), sep="\n")


# ─── 主入口 ──────────────────────────────────────────────────────────────────

def main():
    import argparse
    parser = argparse.ArgumentParser(description="从三大免费数据源抓取矿山企业名单")
    parser.add_argument("--source", default="all", choices=["all", "natural-resources", "safety", "emergency"],
                        help="数据源: all=全部, natural-resources=自然资源厅, safety=国家矿山安监局, emergency=应急管理局")
    parser.add_argument("--provinces", nargs="+", help="只抓指定省份(仅自然资源厅数据源有效)")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--days", type=int, default=0, help="最近N天")
    parser.add_argument("--pages", type=int, default=0, help="每源抓取页数")
    args = parser.parse_args()

    # 组装配置列表
    configs = []
    if args.source in ("all", "natural-resources"):
        configs.extend(NATURAL_RESOURCES_CONFIGS)
    if args.source in ("all", "safety"):
        configs.extend(SAFETY_CONFIGS)
    if args.source in ("all", "emergency"):
        configs.extend(EMERGENCY_CONFIGS)

    # 省份过滤
    if args.provinces:
        configs = [c for c in configs if c.name.replace("应急厅","").replace("安监局","") in args.provinces
                   or c.name in args.provinces]
        if not configs:
            print("❌ 没有匹配的配置")
            sys.exit(1)

    if args.pages > 0:
        for c in configs:
            c.total_pages = args.pages

    all_enterprises = []
    start = time.time()

    for config in configs:
        try:
            all_enterprises.extend(scrape_source(config, days=args.days))
        except Exception as e:
            print(f"\n❌ {config.name}: {e}")
        time.sleep(random.uniform(2, 4))

    # 全局去重
    seen = set()
    unique = []
    for ent in all_enterprises:
        key = (ent.name, ent.province or "全国")
        if key not in seen:
            seen.add(key)
            unique.append(ent)

    output_path = Path(args.output)
    write_csv(unique, output_path)
    print_summary(unique)
    print(f"\n⏱ {time.time()-start:.0f}秒")

    # markdown汇总
    md_path = output_path.with_suffix(".md")
    with open(md_path, "w", encoding="utf-8") as f:
        f.write(f"# 自动抓取的矿山企业名单\n> {datetime.now():%Y-%m-%d %H:%M}  来源: 自然资源厅+安监局+应急管理局\n\n")
        f.write("| 来源 | 企业名称 | 矿种 | 日期 |\n|:---:|:--------|:----|:----|\n")
        for ent in unique:
            f.write(f"| {ent.province or '全国'} | {ent.name} | {ent.mine_type} | {ent.source_date} |\n")
    print(f"📁 {md_path}")


if __name__ == "__main__":
    main()
