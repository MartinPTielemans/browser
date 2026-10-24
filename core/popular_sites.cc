#include "browser_core/popular_sites.h"

#include <string>
#include <string_view>
#include <vector>

#include "base/strings/string_util.h"
#include "base/strings/utf_string_conversions.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// Roughly by how many people visit them worldwide, with a few this browser's users open more moved
// up. The list is built in, asks nothing of the network, and no one pays to be on it.
constexpr PopularSite kSites[] = {
    {"Google", "https://www.google.com/"},
    {"YouTube", "https://www.youtube.com/"},
    {"Facebook", "https://www.facebook.com/"},
    {"Instagram", "https://www.instagram.com/"},
    {"ChatGPT", "https://chatgpt.com/", "OpenAI"},
    {"Wikipedia", "https://www.wikipedia.org/"},
    {"X", "https://x.com/", "Twitter"},
    {"Reddit", "https://www.reddit.com/"},
    {"WhatsApp", "https://web.whatsapp.com/"},
    {"Bing", "https://www.bing.com/"},
    {"Amazon", "https://www.amazon.com/"},
    {"Yahoo", "https://www.yahoo.com/"},
    {"TikTok", "https://www.tiktok.com/"},
    {"Gmail", "https://mail.google.com/"},
    {"Google Maps", "https://www.google.com/maps"},
    {"Baidu", "https://www.baidu.com/"},
    {"Yandex", "https://yandex.com/"},
    {"LinkedIn", "https://www.linkedin.com/"},
    {"GitHub", "https://github.com/"},
    {"Netflix", "https://www.netflix.com/"},
    {"DuckDuckGo", "https://duckduckgo.com/"},
    {"Outlook", "https://outlook.com/", "Hotmail"},
    {"Microsoft 365", "https://www.office.com/", "Office"},
    {"Twitch", "https://www.twitch.tv/"},
    {"Pinterest", "https://www.pinterest.com/"},
    {"Google Drive", "https://drive.google.com/"},
    {"Google Docs", "https://docs.google.com/document/"},
    {"Gemini", "https://gemini.google.com/"},
    {"Claude", "https://claude.ai/", "Anthropic"},
    {"Perplexity", "https://www.perplexity.ai/"},
    {"Microsoft Copilot", "https://copilot.microsoft.com/"},
    {"DeepSeek", "https://chat.deepseek.com/"},
    {"Grok", "https://grok.com/"},
    {"Stack Overflow", "https://stackoverflow.com/"},
    {"eBay", "https://www.ebay.com/"},
    {"Walmart", "https://www.walmart.com/"},
    {"AliExpress", "https://www.aliexpress.com/"},
    {"Etsy", "https://www.etsy.com/"},
    {"Temu", "https://www.temu.com/"},
    {"Spotify", "https://open.spotify.com/"},
    {"Discord", "https://discord.com/app"},
    {"Telegram", "https://web.telegram.org/"},
    {"Messenger", "https://www.messenger.com/"},
    {"Zoom", "https://zoom.us/"},
    {"Google Translate", "https://translate.google.com/"},
    {"Google Calendar", "https://calendar.google.com/"},
    {"Google Sheets", "https://docs.google.com/spreadsheets/"},
    {"Google Meet", "https://meet.google.com/"},
    {"Google Photos", "https://photos.google.com/"},
    {"Google News", "https://news.google.com/"},
    {"Google Flights", "https://www.google.com/travel/flights"},
    {"iCloud", "https://www.icloud.com/"},
    {"Apple", "https://www.apple.com/"},
    {"Microsoft", "https://www.microsoft.com/"},
    {"OneDrive", "https://onedrive.live.com/"},
    {"Dropbox", "https://www.dropbox.com/"},
    {"Notion", "https://www.notion.so/"},
    {"Slack", "https://app.slack.com/"},
    {"Microsoft Teams", "https://teams.microsoft.com/"},
    {"Figma", "https://www.figma.com/"},
    {"Canva", "https://www.canva.com/"},
    {"Trello", "https://trello.com/"},
    {"Asana", "https://app.asana.com/"},
    {"Linear", "https://linear.app/"},
    {"Miro", "https://miro.com/"},
    {"Airtable", "https://airtable.com/"},
    {"Proton Mail", "https://mail.proton.me/"},
    {"Quora", "https://www.quora.com/"},
    {"Tumblr", "https://www.tumblr.com/"},
    {"Threads", "https://www.threads.com/"},
    {"Bluesky", "https://bsky.app/"},
    {"Snapchat", "https://www.snapchat.com/"},
    {"VK", "https://vk.com/"},
    {"Naver", "https://www.naver.com/"},
    {"IMDb", "https://www.imdb.com/"},
    {"Medium", "https://medium.com/"},
    {"Substack", "https://substack.com/"},
    {"Internet Archive", "https://archive.org/", "Wayback Machine"},
    {"Wolfram Alpha", "https://www.wolframalpha.com/"},
    {"Khan Academy", "https://www.khanacademy.org/"},
    {"Coursera", "https://www.coursera.org/"},
    {"Duolingo", "https://www.duolingo.com/"},
    {"Quizlet", "https://quizlet.com/"},
    {"Hacker News", "https://news.ycombinator.com/"},
    {"Product Hunt", "https://www.producthunt.com/"},
    {"GitLab", "https://gitlab.com/"},
    {"npm", "https://www.npmjs.com/"},
    {"MDN Web Docs", "https://developer.mozilla.org/"},
    {"Vercel", "https://vercel.com/"},
    {"Hulu", "https://www.hulu.com/"},
    {"Disney+", "https://www.disneyplus.com/", "Disney Plus"},
    {"Prime Video", "https://www.primevideo.com/"},
    {"HBO Max", "https://www.hbomax.com/"},
    {"Apple Music", "https://music.apple.com/"},
    {"SoundCloud", "https://soundcloud.com/"},
    {"Vimeo", "https://vimeo.com/"},
    {"BBC", "https://www.bbc.com/"},
    {"CNN", "https://www.cnn.com/"},
    {"The New York Times", "https://www.nytimes.com/", "NYT"},
    {"The Guardian", "https://www.theguardian.com/"},
    {"Reuters", "https://www.reuters.com/"},
    {"Bloomberg", "https://www.bloomberg.com/"},
    {"The Washington Post", "https://www.washingtonpost.com/"},
    {"The Wall Street Journal", "https://www.wsj.com/", "WSJ"},
    {"Al Jazeera", "https://www.aljazeera.com/"},
    {"ESPN", "https://www.espn.com/"},
    {"The Verge", "https://www.theverge.com/"},
    {"The Weather Channel", "https://weather.com/"},
    {"Booking.com", "https://www.booking.com/"},
    {"Airbnb", "https://www.airbnb.com/"},
    {"Expedia", "https://www.expedia.com/"},
    {"Tripadvisor", "https://www.tripadvisor.com/"},
    {"Skyscanner", "https://www.skyscanner.net/"},
    {"Yelp", "https://www.yelp.com/"},
    {"PayPal", "https://www.paypal.com/"},
    {"Coinbase", "https://www.coinbase.com/"},
    {"Steam", "https://store.steampowered.com/"},
    {"Roblox", "https://www.roblox.com/"},
    {"Chess.com", "https://www.chess.com/"},
    {"Craigslist", "https://www.craigslist.org/"},
    {"Zillow", "https://www.zillow.com/"},
    {"Indeed", "https://www.indeed.com/"},
    {"Goodreads", "https://www.goodreads.com/"},
    {"Target", "https://www.target.com/"},
    {"Best Buy", "https://www.bestbuy.com/"},
    {"IKEA", "https://www.ikea.com/"},
    {"Speedtest", "https://www.speedtest.net/"},
};

// Whether |name|, from its start or from the start of one of its words, starts with |text|, in
// any case.
bool AnyWordStartsWith(std::string_view name, std::string_view text) {
  size_t start = 0;
  while (!base::StartsWith(name.substr(start), text, base::CompareCase::INSENSITIVE_ASCII)) {
    start = name.find(' ', start);
    if (start == std::string_view::npos) {
      return false;
    }
    ++start;
  }
  return true;
}

}  // namespace

std::vector<PopularSite> PopularSitesFor(std::u16string_view typed) {
  std::string text =
      base::ToLowerASCII(base::UTF16ToUTF8(base::TrimWhitespace(typed, base::TRIM_LEADING)));
  // What a host can be typed with.
  for (std::string_view prefix : {"https://", "http://", "www."}) {
    if (text.starts_with(prefix)) {
      text.erase(0, prefix.size());
    }
  }
  std::vector<PopularSite> sites;
  if (text.empty()) {
    return sites;
  }
  for (const PopularSite& site : kSites) {
    const GURL url(site.url);
    std::string_view host = url.host();
    if (host.starts_with("www.")) {
      host.remove_prefix(4);
    }
    if (host.starts_with(text) || AnyWordStartsWith(site.name, text) ||
        AnyWordStartsWith(site.also, text)) {
      sites.push_back(site);
    }
  }
  return sites;
}

}  // namespace browser_core
