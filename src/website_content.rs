#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebsiteContent {
    pub campaigns: Vec<CampaignContent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignContent {
    pub title: String,
    pub date_range: DateRange,
    pub overview: CampaignOverview,
    pub performance: PerformanceSection,
    pub thesis_scoreboard: ThesisScoreboard,
    pub detail_report: DetailReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    pub const fn new(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DateRange {
    pub start: CalendarDate,
    pub end: CalendarDate,
}

impl DateRange {
    pub const fn new(start: CalendarDate, end: CalendarDate) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CampaignOverview {
    pub eyebrow: String,
    pub headline: String,
    pub summary: String,
    pub context: Vec<String>,
    pub key_metrics: Vec<KeyMetric>,
    pub rules: Vec<String>,
    pub takeaway: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyMetric {
    pub label: String,
    pub value: MetricValue,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetricValue {
    Text(String),
    Percentage(PercentageFigure),
    DateRange(DateRange),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PercentageFigure {
    pub basis_points: i32,
    pub unit: PercentageUnit,
}

impl PercentageFigure {
    pub const fn percent(basis_points: i32) -> Self {
        Self {
            basis_points,
            unit: PercentageUnit::Percent,
        }
    }

    pub const fn percentage_points(basis_points: i32) -> Self {
        Self {
            basis_points,
            unit: PercentageUnit::PercentagePoints,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PercentageUnit {
    Percent,
    PercentagePoints,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformanceSection {
    pub title: String,
    pub summary: String,
    pub chart: ChartSlot,
    pub comparisons: Vec<PerformanceComparison>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartSlot {
    pub source: ChartSource,
    pub prominence: Prominence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChartSource {
    ExistingPerformanceChart,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Prominence {
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformanceComparison {
    pub label: String,
    pub value: PercentageFigure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThesisScoreboard {
    pub title: String,
    pub summary: String,
    pub rows: Vec<ThesisRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThesisRow {
    pub title: String,
    pub realized_return: Option<PercentageFigure>,
    pub visible_summary: String,
    pub tone: ResultTone,
    pub detail_topic: Option<DetailTopic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultTone {
    Positive,
    Mixed,
    Negative,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DetailTopic {
    Wars,
    ScotusTariffRuling,
    GoldSilverUsCredibility,
    TechAi,
    Experts,
    StoppingWhenTheEdgeIsGone,
    Lessons,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetailReport {
    pub title: String,
    pub summary: String,
    pub default_disclosure: DisclosureState,
    pub sections: Vec<DetailSection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetailSection {
    pub topic: DetailTopic,
    pub title: String,
    pub summary: String,
    pub default_disclosure: DisclosureState,
    pub blocks: Vec<DetailBlock>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisclosureState {
    Collapsed,
    Expanded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DetailBlock {
    Paragraph(String),
    OrderedList(Vec<String>),
    UnorderedList(Vec<ListItem>),
    Subsection(DetailSubsection),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetailSubsection {
    pub title: String,
    pub blocks: Vec<DetailBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ListItem {
    Text(String),
    Code(String),
}

pub fn website_content() -> WebsiteContent {
    WebsiteContent {
        campaigns: vec![campaign_1_content()],
    }
}

pub fn campaign_1_content() -> CampaignContent {
    let date_range = DateRange::new(
        CalendarDate::new(2025, 10, 28),
        CalendarDate::new(2026, 4, 28),
    );

    CampaignContent {
        title: text("Ocotelolco Campaign 1"),
        date_range,
        overview: CampaignOverview {
            eyebrow: text("Campaign 1"),
            headline: text("Ocotelolco Campaign 1"),
            summary: text(
                "From October 28, 2025 to April 28, 2026, I ran a six-month experiment: could I actively trade around my own forecasts and beat the S&P 500?",
            ),
            context: vec![
                text("I wanted to know whether I could turn my forecasts into market-beating trades because forecasting and prediction have been interests of mine for a long time. If I was good at using forecasts to make trades, it would be useful to learn that early. If I was bad at it, it would be useful to learn that too, and then stop wasting time on it."),
                text("My event calls were mostly right, but that did not automatically make them easy trades. Bitcoin fell. The US attacked Venezuela and then Iran. SCOTUS ruled against tariffs. The tech thesis was messier: tech ended higher by the end of the campaign, but for much of the six-month period it was flat or down, and my profits there came more from timing than from a simple sector rally."),
            ],
            key_metrics: vec![
                KeyMetric {
                    label: text("Final return vs S&P 500"),
                    value: MetricValue::Percentage(PercentageFigure::percentage_points(788)),
                    note: None,
                },
                KeyMetric {
                    label: text("High vs S&P 500"),
                    value: MetricValue::Percentage(PercentageFigure::percentage_points(2107)),
                    note: None,
                },
                KeyMetric {
                    label: text("Low vs S&P 500"),
                    value: MetricValue::Percentage(PercentageFigure::percentage_points(-146)),
                    note: None,
                },
            ],
            rules: vec![
                text("Do not lose money."),
                text("Beat the S&P 500."),
                text("Make enough calls and enough trades that the result was not just a few lucky calls."),
            ],
            takeaway: text("The major calls were right: Bitcoin fell, war risk materialized, SCOTUS undercut tariffs, and the instability thesis paid. But being right was still only half the game. Making money depended just as much on sizing, timing, instrument choice, and exiting when the trade stopped matching the thesis."),
        },
        performance: PerformanceSection {
            title: text("Performance"),
            summary: text(
                "Portfolio balance and S&P 500 comparison for the full campaign window.",
            ),
            chart: ChartSlot {
                source: ChartSource::ExistingPerformanceChart,
                prominence: Prominence::Primary,
            },
            comparisons: vec![
                PerformanceComparison {
                    label: text("Ocotelolco Campaign 1"),
                    value: PercentageFigure::percent(1075),
                },
                PerformanceComparison {
                    label: text("S&P 500"),
                    value: PercentageFigure::percent(360),
                },
            ],
        },
        thesis_scoreboard: ThesisScoreboard {
            title: text("Thesis Scoreboard"),
            summary: text(
                "The high-level relative returns by thesis, from my ticker-tag analysis.",
            ),
            rows: vec![
                ThesisRow {
                    title: text("US credibility / instability"),
                    realized_return: Some(PercentageFigure::percent(400)),
                    visible_summary: text("Gold and silver were the strongest expression of the instability thesis."),
                    tone: ResultTone::Positive,
                    detail_topic: Some(DetailTopic::GoldSilverUsCredibility),
                },
                ThesisRow {
                    title: text("Bitcoin down"),
                    realized_return: Some(PercentageFigure::percent(180)),
                    visible_summary: text("Bitcoin fell, but this stayed a scoreboard item rather than a major long-form section."),
                    tone: ResultTone::Positive,
                    detail_topic: None,
                },
                ThesisRow {
                    title: text("Tariffs"),
                    realized_return: Some(PercentageFigure::percent(170)),
                    visible_summary: text("The tariff basket worked better than it felt in real time."),
                    tone: ResultTone::Positive,
                    detail_topic: Some(DetailTopic::ScotusTariffRuling),
                },
                ThesisRow {
                    title: text("War goes bad"),
                    realized_return: Some(PercentageFigure::percent(130)),
                    visible_summary: text("War trades were directionally useful, but market timing was complicated."),
                    tone: ResultTone::Mixed,
                    detail_topic: Some(DetailTopic::Wars),
                },
                ThesisRow {
                    title: text("War on horizon"),
                    realized_return: Some(PercentageFigure::percent(120)),
                    visible_summary: text("Buying well before expected conflicts mostly worked better than reacting after outbreak."),
                    tone: ResultTone::Positive,
                    detail_topic: Some(DetailTopic::Wars),
                },
                ThesisRow {
                    title: text("Tech"),
                    realized_return: Some(PercentageFigure::percent(120)),
                    visible_summary: text("The AI thesis was profitable, but not as simple as broad tech going up."),
                    tone: ResultTone::Mixed,
                    detail_topic: Some(DetailTopic::TechAi),
                },
                ThesisRow {
                    title: text("Retreat"),
                    realized_return: Some(PercentageFigure::percent(-40)),
                    visible_summary: text("Late-campaign defensive trading gave back some of the earlier lead."),
                    tone: ResultTone::Negative,
                    detail_topic: Some(DetailTopic::StoppingWhenTheEdgeIsGone),
                },
                ThesisRow {
                    title: text("Regions"),
                    realized_return: Some(PercentageFigure::percent(-230)),
                    visible_summary: text("Regional ETFs behaved like risk assets, not like safer broad index substitutes."),
                    tone: ResultTone::Negative,
                    detail_topic: Some(DetailTopic::StoppingWhenTheEdgeIsGone),
                },
            ],
        },
        detail_report: DetailReport {
            title: text("Full Report"),
            summary: text("Long-form writings on my trades, context, caveats, and lessons"),
            default_disclosure: DisclosureState::Collapsed,
            sections: vec![
                wars_section(),
                tariff_section(),
                credibility_section(),
                tech_section(),
                experts_section(),
                stopping_section(),
                lessons_section(),
            ],
        },
    }
}

fn wars_section() -> DetailSection {
    detail_section(
        DetailTopic::Wars,
        "Wars",
        "How the Venezuela and Iran war theses were formed, traded, and revised.",
        vec![
            paragraph("My war thesis started with Venezuela and then moved to Iran. Around October, prediction markets had the odds of a US attack on Venezuela around 40%. Forecasters on Metaculus were more like 70%, and some of the top forecasters I followed were closer to 80%. I trusted the forecasters more than the general market, and even 40% looked high relative to ordinary public discussion."),
            paragraph("I should also give a shout-out to the Sentinal Global Risk forecasting group. Their weekly forecasting newsletter was one of the sources I relied on most heavily for my wars thesis."),
            paragraph("Leading up to Venezuela, I bought oil companies, oil-related exposure, and VIXY, an ETF linked to short-term VIX futures. I did not make defense companies a major part of the strategy. My thinking was that defense company valuations were more likely to reflect longer-term procurement cycles than short-term news about a specific conflict."),
            paragraph("After Venezuela, I moved to anticipating war with Iran. I was worried about picking the wrong vehicle, so I diversified across several things I expected to benefit from a war: oil, oil companies, tanker/shipping exposure, and fertilizer companies. I had used volatility products for the Venezuela trade, but I mostly stopped using them during the Iran war."),
            paragraph("The results were uneven. Oil and energy did well overall. Shipping was directionally right in the market, but my own result there was modest. Fertilizer was a small loss. I think now that war made fertilizer more valuable, but it also made fertilizer more expensive to produce, so the net effect on fertilizer stocks was negative."),
            paragraph("My approach to trading around wars was to buy well in advance of the event and sell when the war actually broke out. That was mostly right. The market often prices fear before the event and then can rise once the event happens, especially if the outcome is less bad than expected."),
            subsection(
                "VIXY",
                vec![
                    paragraph("I traded VIXY frequently. My understanding was that VIXY is usually a terrible buy-and-hold product because it tends to decay over time. It is designed for short-term volatility exposure, not long-term ownership."),
                    paragraph("My actual VIXY trading was not good. I lost money on VIXY and VIXM. I tried to jump in when prediction-market odds of war rose and then exit a few days later. Ironically, I would have been better off buying and holding VIXY through the period, which is usually not a good idea."),
                ],
            ),
            subsection(
                "What War Did To Markets",
                vec![
                    paragraph("My initial model was simple: war is bad for markets because it destroys productive capacity, disrupts trade, raises uncertainty, and simply makes many companies less valuable."),
                    paragraph("That is true, but the timing is not straightforward. If a war is unexpected, markets may fall when the war begins. But if markets already expect a war, they may fall before it begins and then recover once the uncertainty is resolved. If the war is less bad than feared, the market can even rise after the first shots are fired."),
                ],
            ),
        ],
    )
}

fn tariff_section() -> DetailSection {
    detail_section(
        DetailTopic::ScotusTariffRuling,
        "SCOTUS Tariff Ruling",
        "The legal thesis, the stock basket, and the cleaner prediction-market bet I missed.",
        vec![
            paragraph("I used to be a law nerd. In college I read a lot of court opinions and legal scholarship, and I followed law blogs from lawyers, judges, and professors. My view was that Supreme Court justices were mostly sincere believers in their judicial ideologies, and that public coverage often overstated simple partisan explanations."),
            paragraph("That background made me think I had an edge on the tariff case. My view was that the Constitution gives tariff authority to Congress, and that originalism gives a clear answer: the president cannot exercise that power so broadly without Congress. I put the odds of SCOTUS ruling against tariffs around 90%. At the time, prediction-market odds were closer to 50%, so I thought the ruling was not fully priced in."),
            paragraph("My plan was to buy a basket of stocks that I expected to benefit if tariffs went away, mostly consumer staples and consumer goods. I bought before oral arguments on November 5, because I expected the arguments to reveal how the justices were thinking. The odds did move from roughly 50% to 70% after oral arguments, but my basket did not immediately move much, which made me feel like the trade had failed."),
            paragraph("The data told a different story. My tariff tag finished positive, at +1.7%. My subjective impression in real time was that I had not gotten a positive return from these trades. I only realized the result was positive after analyzing the transaction history."),
            subsection(
                "The Cleaner Bet",
                vec![
                    paragraph("The obvious missed opportunity is that there was a prediction market on the exact thing I was trying to express through stocks, and I had a strong opinion of my own. At 50% market odds, a winning prediction-market bet would have doubled the stake before fees. That was a much cleaner instrument than building a tariff-sensitive equity basket."),
                    paragraph("The lesson is not that stock baskets are bad. It is that when a prediction market exists on the exact event I care about, and I have a strong divergent probability estimate, I ought to bet there directly. The war trades also had related prediction markets, but I did not have the same strong probability view of my own there."),
                ],
            ),
            subsection(
                "Originalism Was Messier Than I Expected",
                vec![
                    paragraph("My legal thesis was also messier than I expected. I thought originalist justices would mostly line up against tariffs. That did not really happen."),
                    paragraph("Justice Gorsuch made the kind of originalist argument I expected against tariffs. Justice Thomas, also highly originalist, argued in favor of the delegation. Justice Barrett ruled against tariffs for textualist reasons, but did not address the constitutional question in the same way Gorsuch did. Justice Jackson also ruled against tariffs for reasons closer to Barrett's."),
                    paragraph("The result was not \"originalists all agree.\" It was more complicated. I still think legal ideology matters, but I have evolved toward thinking that judicial opinions are not personal essays. They are instruments built to resolve cases, manage coalitions, avoid unnecessary constitutional questions, and shape doctrine. That helps explain why the justices could reach similar outcomes through different legal reasoning, or disagree even when they share an interpretive label."),
                ],
            ),
        ],
    )
}

fn credibility_section() -> DetailSection {
    detail_section(
        DetailTopic::GoldSilverUsCredibility,
        "Gold, Silver, And US Credibility",
        "Why instability pointed toward precious metals, and why safe-haven assets were more complicated than I expected.",
        vec![
            paragraph("The US credibility thesis was that Trump-era instability would make the world less confident in the US-led system. If people were less sure the US was predictable, they would look for alternatives to dollar-linked assets. The obvious candidates were commodities, especially gold and silver."),
            paragraph("I resisted this at first. When I think of gold and silver, I think of old people getting scammed by ads on Fox News into buying or selling gold. If I bought gold and lost money, I would feel embarrassed. But the more I looked into it, the more reasonable the trade seemed. Someone smart I trust who worked in finance also spoke highly of trading gold, which made me take the idea more seriously. The world relies heavily on the dollar, and the dollar relies on confidence in US institutions. If that confidence weakens, it makes sense for some investors and central banks to diversify into gold."),
            paragraph("This became my strongest thesis by tag. The \"US credibility\" basket finished +4.0%. The gold and silver pieces were a major part of that, especially IAU and SLV. SILJ also helped, while GDX was basically flat."),
            paragraph("Gold and silver did very well for me, but the trade also became volatile. In February, shortly after my second daughter was born, I checked the market at 9:30 AM after being awake most of the night and saw my gold exposure drop sharply at the open. I remember seeing something like a 16% drop and immediately sold everything. No stress, no denial, just the simple realization that I was completely unprepared to deal with that kind of move and still had the opportunity to come out ahead."),
            subsection(
                "The Irony Of Gold",
                vec![
                    paragraph("Gold is often described as a safe-haven asset. That is true, but the Iran war made me think about what \"safe haven\" actually means."),
                    paragraph("That is the irony. People buy gold when they are worried about instability and the dollar system. But when instability actually arrives, governments, institutions, and regular people may sell gold because it is one of the few things liquid enough to turn into dollars quickly. Even when the original fear is about trusting dollars, the emergency move can be to get dollars. At least, that is how I am thinking about it now, and I could be wrong."),
                ],
            ),
        ],
    )
}

fn tech_section() -> DetailSection {
    detail_section(
        DetailTopic::TechAi,
        "Tech And AI",
        "Why the AI thesis focused on software, cloud, small companies, and compute infrastructure.",
        vec![
            paragraph("I believe in AI. It is already useful to me as a programmer, and I think we are early in a long period where people and companies will keep discovering great new ways to use it."),
            paragraph("But believing in AI is not the same thing as believing every AI stock is a good buy. The dot-com bubble happened even though the internet really was transformative. A technology can be real, important, and world-changing while many of the companies attached to it still become overvalued and crash."),
            paragraph("My working assumption was that software demand is inelastic. There is a lot of software people and businesses want but cannot currently afford to build. If AI makes software cheaper to produce, there should be more software, not less. That made me more interested in software companies, small companies, cloud infrastructure, and raw materials for compute than in simply buying the most obvious AI winners."),
            paragraph("During this period, I became more skeptical that the current frontier model companies are necessarily the best stock-market expression of AI. Companies like OpenAI and Anthropic sell access to the best models, but open source models keep trailing them by only a few months for many use cases. For high-volume tasks that do not require frontier intelligence, cheaper open models may be good enough."),
            paragraph("I also became more cautious about Nvidia and other current AI hardware winners. Their valuation depends heavily on GPUs staying essential to AI, and that may not turn out to be true. Future models may be less dependent on GPU architecture. Nvidia's CUDA ecosystem is a real advantage, though maybe a fragile one, but competitors may improve their software ecosystems, and at some price the economics of non-CUDA systems may matter more than compatibility."),
            paragraph("The analogy I kept coming back to was electricity. Since electricity was invented, which grew more: electricity companies, or everyone who learned how to use electricity? I am pretty sure the second. If AI is like that, then the biggest winners may not be the model companies themselves, but the companies that use AI to become more productive."),
            paragraph("My tech investments included:"),
            ordered_list(&[
                "Small and mid-sized software companies through XSW.",
                "Cloud computing companies through SKYY.",
                "The Russell 2000 as a broad bet on smaller companies benefiting from cheaper software.",
                "Materials and infrastructure connected to compute, including copper and electricity generation.",
            ]),
            paragraph("The tech tag finished positive, at +1.2%. That was profitable, but the thesis was not as clean as \"tech went up.\" Tech struggled for much of the campaign, and I made money because of timing and trade selection."),
        ],
    )
}

fn experts_section() -> DetailSection {
    detail_section(
        DetailTopic::Experts,
        "Experts",
        "Why domain expertise helped with facts but did not replace probabilistic trade construction.",
        vec![
            paragraph("I listened to a lot of oil experts because oil was one of my main ways to trade the war thesis. They were useful on details: what happens to an oil well if pumping stops, how hard it is to restart production, what routes can substitute for Hormuz, and how oil logistics actually work."),
            paragraph("But they were less useful on the whole trade. Oil expertise does not automatically tell you the probability that Iran will strike Gulf energy infrastructure, how markets will price that risk, or how quickly politicians will change their posture. Several experts seemed confident. But what they knew were narrow facts about their domain, not the full chain of uncertainty."),
            paragraph("I saw a similar pattern with tankers. I spoke with a tanker expert who seemed deeply immersed in tanker forums and industry news. He had a stock strategy built around tankers and a lot of conviction. But he also treated some major geopolitical contingencies as afterthoughts, even when he himself acknowledged those contingencies were central to the trade."),
            paragraph("The market example here is BWET. I did not buy BWET, but it showed how much tanker stocks could move. The Breakwave Tanker Shipping ETF reportedly rose about 1,300% over the prior year and kept rising up to the Iran war."),
            paragraph("The point is not that the tanker expert was obviously wrong about the industry. It is that industry-specific knowledge was not enough to make money if the timing was bad."),
            paragraph("The missing skill is probabilistic translation. Forecasting and prediction-market communities train you to convert information, uncertainty, and your own confidence into quantitative probabilities. An expert can know a ton about a domain and still have a subjective feeling that is poorly calibrated. The trade is almost never about one variable. It is about the joint distribution of politics, logistics, timing, market expectations, and other investors' positioning."),
        ],
    )
}

fn stopping_section() -> DetailSection {
    detail_section(
        DetailTopic::StoppingWhenTheEdgeIsGone,
        "Stopping When The Edge Is Gone",
        "The expensive late-campaign period after the original theses had resolved.",
        vec![
            paragraph("After the Iran war started, my main theses had resolved. I was out of good ideas."),
            paragraph("The correct thing to do was probably to stop trading and move into an index fund. But this was only four months into a six-month campaign, and one of my goals was to trade a lot. I felt real internal conflict about that. Stopping early felt like abandoning the experiment, but continuing without a clear edge also felt wrong. I kept looking for trades partly because I wanted the campaign to remain active and partly because I thought even bad trades might teach me something if I was trying my best."),
            paragraph("That was expensive."),
            paragraph("At the outbreak of the Iran war, I was roughly 22 percentage points ahead of the S&P 500. I finished 7.88 points ahead. Almost all of the lost edge came after my original theses were exhausted."),
            paragraph("The experience felt completely different before and after the Iran war. Before the war, I felt like I had deeply thought-through plans and was months ahead of the news. I was mostly alone, thinking about issues almost no one was talking about. After the war started, everyone was talking about those issues, and I was mostly scrambling and chasing headlines."),
            paragraph("I put money into regional ETFs like Vietnam, Israel, Mexico, and Latin America, thinking that entire regions were diversified enough to function like broad index exposure. That was wrong. Regional ETFs are not just mini-S&P 500s. They are risk assets that investors often abandon faster during global drawdowns. My regions tag finished negative, at -2.3%."),
            paragraph("I also bet against the market after deciding the Iran war was going badly, and would get worse. That worked for a few weeks, but then the market recovered quickly after Trump announced that the war was effectively over. Those trades ended up mixed to negative."),
            paragraph("The point is not \"never keep trading.\" The campaign should have had a rule for what to do when the original edge disappeared. If I have no active thesis, the default should be the index. Continuing to trade because the experiment is not over is not the same as having an edge."),
        ],
    )
}

fn lessons_section() -> DetailSection {
    detail_section(
        DetailTopic::Lessons,
        "Lessons",
        "The distilled takeaways from the campaign.",
        vec![
            subsection(
                "Prediction Is Not Trade Construction",
                vec![paragraph("I was right about several events and still made mistakes expressing those views. There is a difference between predicting the world and choosing the right instrument, entry date, exit date, and position size.")],
            ),
            subsection(
                "Subjective Trading Results Feel Unreliable",
                vec![paragraph("My tariff trades felt disappointing in real time but were profitable when I reviewed the transactions. Watching red and green numbers after individual clicks is not the same thing as analyzing the whole strategy.")],
            ),
            subsection(
                "The Cleanest Instrument Usually Deserves A Look",
                vec![paragraph("For tariffs, I tried to express an event prediction through stocks even though there was a prediction market on the exact event and I had a strong probability estimate of my own. At 50% odds, a winning event-market bet would have doubled before fees. I should have bet there directly. Related prediction markets existed for the war trades too, but I did not have the same strong independent probability view there.")],
            ),
            subsection(
                "Experts Need To Be Mapped To The Right Question",
                vec![paragraph("Oil experts were helpful about oil mechanics. Tanker experts were helpful about tanker mechanics. But the trade required probabilities about war, infrastructure attacks, policy reaction, market positioning, and timing. Narrow expertise helped, but it was not enough.")],
            ),
        ],
    )
}

fn detail_section(
    topic: DetailTopic,
    title: &str,
    summary: &str,
    blocks: Vec<DetailBlock>,
) -> DetailSection {
    DetailSection {
        topic,
        title: text(title),
        summary: text(summary),
        default_disclosure: DisclosureState::Collapsed,
        blocks,
    }
}

fn paragraph(value: &str) -> DetailBlock {
    DetailBlock::Paragraph(text(value))
}

fn ordered_list(items: &[&str]) -> DetailBlock {
    DetailBlock::OrderedList(items.iter().map(|item| text(item)).collect())
}

fn unordered_list(items: Vec<ListItem>) -> DetailBlock {
    DetailBlock::UnorderedList(items)
}

fn subsection(title: &str, blocks: Vec<DetailBlock>) -> DetailBlock {
    DetailBlock::Subsection(DetailSubsection {
        title: text(title),
        blocks,
    })
}

fn text(value: &str) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_1_keeps_performance_chart_primary() {
        let content = campaign_1_content();

        assert_eq!(
            content.performance.chart.source,
            ChartSource::ExistingPerformanceChart
        );
        assert_eq!(content.performance.chart.prominence, Prominence::Primary);
    }

    #[test]
    fn campaign_1_overview_metrics_are_final_high_low() {
        let content = campaign_1_content();

        assert_eq!(content.overview.key_metrics.len(), 3);
        assert_eq!(
            content.overview.key_metrics[0].label,
            "Final return vs S&P 500"
        );
        assert_eq!(
            content.overview.key_metrics[0].value,
            MetricValue::Percentage(PercentageFigure::percentage_points(788))
        );
        assert_eq!(content.overview.key_metrics[1].label, "High vs S&P 500");
        assert_eq!(
            content.overview.key_metrics[1].value,
            MetricValue::Percentage(PercentageFigure::percentage_points(2107))
        );
        assert_eq!(content.overview.key_metrics[2].label, "Low vs S&P 500");
        assert_eq!(
            content.overview.key_metrics[2].value,
            MetricValue::Percentage(PercentageFigure::percentage_points(-146))
        );
    }

    #[test]
    fn campaign_1_details_default_to_collapsed() {
        let content = campaign_1_content();

        assert_eq!(
            content.detail_report.default_disclosure,
            DisclosureState::Collapsed
        );
        assert!(content
            .detail_report
            .sections
            .iter()
            .all(|section| section.default_disclosure == DisclosureState::Collapsed));
    }

    #[test]
    fn detail_sections_contain_revised_report_body() {
        let content = campaign_1_content();

        assert_eq!(content.detail_report.sections.len(), 7);
        assert_eq!(
            content.detail_report.summary,
            "Long-form writings on my trades, context, caveats, and lessons"
        );
        assert!(content
            .detail_report
            .sections
            .iter()
            .all(|section| !section.blocks.is_empty()));
        assert!(!content
            .detail_report
            .sections
            .iter()
            .any(|section| section.title == "Sources And Notes"));
        assert!(content.detail_report.sections.iter().any(|section| {
            section.topic == DetailTopic::ScotusTariffRuling
                && section.blocks.iter().any(|block| {
                    matches!(
                        block,
                        DetailBlock::Subsection(subsection)
                            if subsection.title == "The Cleaner Bet"
                    )
                })
        }));
    }

    #[test]
    fn thesis_rows_link_to_optional_detail_sections() {
        let content = campaign_1_content();
        let detail_topics = content
            .detail_report
            .sections
            .iter()
            .map(|section| section.topic)
            .collect::<std::collections::BTreeSet<_>>();

        for row in &content.thesis_scoreboard.rows {
            if let Some(detail_topic) = row.detail_topic {
                assert!(detail_topics.contains(&detail_topic));
            }
        }
    }
}
