# UX Laws — 30 Laws from lawsofux.com

## Performance (2 laws)

### UX-006: Doherty Threshold

Productivity soars when a computer and its users interact at a pace (<400ms) that ensures that neither has to wait on the other.

**Principle:** System feedback within 400ms keeps users' attention and increases productivity. Use perceived performance to improve response time perception. Animation can visually engage users during background processing. Progress bars help make wait times tolerable. Purposefully adding a brief delay can increase perceived value and trust.

**Application:** Target <400ms response time for all interactions. Use skeleton screens and loading animations for longer operations. Implement optimistic UI updates. Show progress indicators for multi-step processes. Pre-fetch data when user intent is predictable. Use perceived performance techniques when actual speed isn't achievable.

[Reference](https://lawsofux.com/doherty-threshold/)

### UX-007: Fitts's Law

The time to acquire a target is a function of the distance to and size of the target.

**Principle:** Fast movements and small targets result in greater error rates due to the speed-accuracy trade-off. Touch targets should be large enough for accurate selection, have ample spacing, and be placed in easily acquirable areas of the interface.

**Application:** Make interactive elements at least 44px (touch) or 24px (pointer). Increase spacing between touch targets. Place primary actions in easily reachable areas. Use edge and corner targets on desktop (infinite depth). Make destructive actions harder to reach than constructive ones. Size buttons proportionally to their importance.

[Reference](https://lawsofux.com/fittss-law/)

## Decision (4 laws)

### UX-002: Choice Overload

The tendency for people to get overwhelmed when presented with a large number of options, often used interchangeably with the term paradox of choice.

**Principle:** Too many options hurt users' decision-making ability and can significantly impact how they feel about the experience. When comparison is necessary, enable side-by-side comparison of related items. Optimize designs for the decision-making process by prioritizing content shown at any given moment.

**Application:** Limit the number of choices presented at once. Use progressive disclosure to reveal options as needed. Provide search and filtering tools to narrow choices. Feature recommended or popular options prominently. Use side-by-side comparison views for complex decisions like pricing tiers.

[Reference](https://lawsofux.com/choice-overload/)

### UX-004: Cognitive Bias

A systematic error of thinking or rationality in judgment that influences our perception of the world and our decision-making ability.

**Principle:** We conserve mental energy by developing rules of thumb (heuristics) based on past experiences. These mental shortcuts increase efficiency but can influence decision-making without our awareness. Understanding our intrinsic biases may not eliminate them but increases the chance of identifying them.

**Application:** Design with awareness of confirmation bias, anchoring, and other cognitive biases. Present balanced information to counteract bias. Use data-driven defaults that account for known biases. Test designs with diverse user groups. Be ethical — don't exploit cognitive biases to manipulate users.

[Reference](https://lawsofux.com/cognitive-bias/)

### UX-010: Hick's Law

The time it takes to make a decision increases with the number and complexity of choices.

**Principle:** More stimuli to choose from means longer decision time. Users bombarded with choices must take time to interpret and decide. Minimize choices when response times are critical. Break complex tasks into smaller steps. Use progressive onboarding for new users.

**Application:** Reduce navigation options to essential items. Use progressive disclosure for complex features. Highlight recommended options to guide decisions. Break multi-step processes into simple stages. Be careful not to over-simplify to the point of abstraction. Use categorization to organize large option sets.

[Reference](https://lawsofux.com/hicks-law/)

### UX-017: Mental Model

A compressed model based on what we think we know about a system and how it works.

**Principle:** Users form working models of systems and apply them to new similar situations. Match designs to users' mental models to improve experience, enabling knowledge transfer between products without needing to learn how new systems work. Shrinking the gap between designer and user mental models is one of the biggest UX challenges.

**Application:** Conduct user research to understand mental models. Use familiar metaphors and patterns (e.g., shopping cart for e-commerce). Follow platform-specific conventions. Use card sorting to understand user categorization. Create personas and journey maps. Test with real users to validate alignment.

[Reference](https://lawsofux.com/mental-model/)

## Memory (7 laws)

### UX-003: Chunking

A process by which individual pieces of an information set are broken down and then grouped together in a meaningful whole.

**Principle:** Chunking enables users to easily scan content and identify information aligned with their goals. Structuring content into visually distinct groups with clear hierarchy enables designers to align information with how people evaluate and process digital content.

**Application:** Break long content into digestible groups. Use visual separators (whitespace, borders, backgrounds) to define chunks. Apply clear hierarchy within and between groups. Format phone numbers, credit cards, and codes in chunks. Organize navigation into logical groups.

[Reference](https://lawsofux.com/chunking/)

### UX-005: Cognitive Load

The amount of mental resources needed to understand and interact with an interface.

**Principle:** When information exceeds available mental capacity, tasks become more difficult, details are missed, and users feel overwhelmed. Intrinsic cognitive load refers to effort for goal-relevant information. Extraneous cognitive load refers to processing that doesn't help understanding (e.g., distracting design elements).

**Application:** Minimize extraneous cognitive load by removing unnecessary elements. Use progressive disclosure to reveal complexity gradually. Maintain consistency in UI patterns. Reduce the number of decisions required. Use familiar patterns and conventions. Keep forms simple with clear labels.

[Reference](https://lawsofux.com/cognitive-load/)

### UX-018: Miller's Law

The average person can only keep 7 (plus or minus 2) items in their working memory.

**Principle:** Short-term memory capacity is limited to approximately 7 items (5-9 range). Don't use the 'magical number seven' to justify unnecessary design limitations. Organize content into smaller chunks for easier processing. Remember that capacity varies per individual based on prior knowledge and context.

**Application:** Organize navigation into groups of 5-9 items maximum. Chunk phone numbers, codes, and long strings. Don't use this as rigid rule — it's a guideline. Structure information hierarchically. Use progressive disclosure for complex content. Provide visual aids to support memory.

[Reference](https://lawsofux.com/millers-law/)

### UX-025: Selective Attention

The process of focusing attention only to a subset of stimuli in the environment — usually those related to our goals.

**Principle:** People filter out information that isn't relevant to maintain focus on important information. Banner Blindness demonstrates this — users ignore content resembling ads. Change Blindness occurs when significant changes go unnoticed due to attention limitations and lack of strong cues.

**Application:** Guide users' attention with visual hierarchy. Avoid styling content to look like ads (Banner Blindness). Use clear visual cues for important changes. Avoid competing changes happening simultaneously. Don't rely on users noticing passive updates — use active notifications. Design for focused task completion.

[Reference](https://lawsofux.com/selective-attention/)

### UX-026: Serial Position Effect

Users have a propensity to best remember the first and last items in a series.

**Principle:** The primacy effect (first items) and recency effect (last items) explain why items at the beginning and end of sequences are recalled more accurately than middle items. Placing least important items in the middle is helpful as they tend to be stored less frequently in memory.

**Application:** Place the most important navigation items at the beginning and end. Put key actions at the far left and right of toolbars. Place critical information at the start and end of lists. Use the middle for less critical content. Apply to tab bars, navigation menus, and feature lists.

[Reference](https://lawsofux.com/serial-position-effect/)

### UX-028: Von Restorff Effect

When multiple similar objects are present, the one that differs from the rest is most likely to be remembered. Also known as The Isolation Effect.

**Principle:** Make important information or key actions visually distinctive. Use restraint when placing emphasis to avoid elements competing with each other and salient items being mistaken for ads. Don't exclude users with color vision deficiency by relying exclusively on color for contrast.

**Application:** Visually distinguish CTAs from surrounding elements. Use color, size, or shape to highlight key content. Don't overuse visual emphasis — it loses effectiveness. Ensure accessibility (don't rely on color alone). Use multiple visual cues (color + size + position). Consider motion sensitivity when using animation for emphasis.

[Reference](https://lawsofux.com/von-restorff-effect/)

### UX-029: Working Memory

A cognitive system that temporarily holds and manipulates information needed to complete tasks.

**Principle:** Working memory is limited to 4-7 chunks that fade after 20-30 seconds. Our brains recognize previously seen information better than recalling new information. Support recognition over recall by making viewed information clear. Place memory burden on the system, not the user.

**Application:** Carry information across screens when needed (e.g., comparison tables). Visually differentiate visited links. Provide breadcrumbs for navigation context. Show recently viewed items. Keep related information visible simultaneously. Use persistent UI elements for critical context. Minimize required memorization between steps.

[Reference](https://lawsofux.com/working-memory/)

## Gestalt (5 laws)

### UX-012: Law of Common Region

Elements tend to be perceived into groups if they are sharing an area with a clearly defined boundary.

**Principle:** Common region creates clear structure and helps users quickly understand relationships between elements and sections. Adding borders or defining backgrounds around elements are effective ways to create common region groupings.

**Application:** Use cards to group related content. Apply borders around related form fields. Use background colors to distinguish sections. Create visual containers for related actions. Maintain consistent boundary treatments across similar groupings.

[Reference](https://lawsofux.com/law-of-common-region/)

### UX-013: Law of Proximity

Objects that are near, or proximate to each other, tend to be grouped together.

**Principle:** Proximity establishes relationships with nearby objects. Elements in close proximity are perceived to share similar functionality or traits. Proximity helps users understand and organize information faster and more efficiently.

**Application:** Place related form labels close to their inputs. Group related navigation items together. Use spacing to separate distinct content sections. Keep action buttons near the content they affect. Organize dashboard cards by proximity to indicate relationships.

[Reference](https://lawsofux.com/law-of-proximity/)

### UX-014: Law of Pragnanz

People will perceive and interpret ambiguous or complex images as the simplest form possible, because it is the interpretation that requires the least cognitive effort.

**Principle:** The human eye likes to find simplicity and order in complex shapes to prevent information overwhelm. People are better able to visually process and remember simple figures than complex ones. Complex shapes are simplified by transforming them into single, unified shapes.

**Application:** Simplify visual elements and icons. Use clean, geometric shapes. Reduce visual noise and unnecessary detail. Create clear visual hierarchy. Avoid ambiguous visual arrangements. Test icon and symbol comprehension with users.

[Reference](https://lawsofux.com/law-of-pr%C3%A4gnanz/)

### UX-015: Law of Similarity

The human eye tends to perceive similar elements as a complete picture, shape, or group, even if those elements are separated.

**Principle:** Visually similar elements are perceived as related. Color, shape, size, orientation, and movement can signal group membership and shared meaning or functionality. Links and navigation must be visually differentiated from normal text.

**Application:** Use consistent visual styling for elements with same function. Differentiate interactive elements from static content. Apply consistent color coding across the interface. Ensure navigation items look distinctly different from body text. Use consistent icon styles within the same context.

[Reference](https://lawsofux.com/law-of-similarity/)

### UX-016: Law of Uniform Connectedness

Elements that are visually connected are perceived as more related than elements with no connection.

**Principle:** Group functions of similar nature so they are visually connected via colors, lines, frames, or other shapes. Tangible connecting references (lines, arrows) from one element to the next also create visual connection. Use uniform connectedness to show context or emphasize relationships.

**Application:** Use lines or arrows to connect related elements. Apply consistent background colors to related groups. Use borders to visually link content (like Google's featured snippets). Create visual flow between sequential steps. Connect labels to their form fields with visual cues.

[Reference](https://lawsofux.com/law-of-uniform-connectedness/)

## Behavior (6 laws)

### UX-008: Flow

The mental state in which a person performing some activity is fully immersed in a feeling of energized focus, full involvement, and enjoyment in the process of the activity.

**Principle:** Flow occurs when there is balance between task difficulty and skill level, characterized by intense focused concentration and a sense of control. A task too difficult leads to frustration; too easy leads to boredom. Design for flow by providing feedback and removing unnecessary friction.

**Application:** Match challenge level to user skill. Provide clear, immediate feedback for every action. Remove friction and unnecessary steps. Make content and features discoverable. Ensure system responsiveness. Allow users to maintain focus without interruptions. Use progressive complexity.

[Reference](https://lawsofux.com/flow/)

### UX-009: Goal-Gradient Effect

The tendency to approach a goal increases with proximity to the goal.

**Principle:** The closer users are to completing a task, the faster they work toward it. Providing artificial progress toward a goal helps ensure users have motivation to complete the task. Clear progress indication motivates task completion.

**Application:** Show progress indicators that emphasize how far users have come. Use progress bars in multi-step flows. Pre-fill initial progress to create momentum (e.g., loyalty cards with starter stamps). Break large tasks into visible milestones. Celebrate near-completion to maintain motivation.

[Reference](https://lawsofux.com/goal-gradient-effect/)

### UX-020: Paradox of the Active User

Users never read manuals but start using the software immediately.

**Principle:** Users are motivated to complete immediate tasks and won't spend time reading documentation upfront. The paradox is that users would save time long-term by learning the system first. Make guidance accessible throughout the product and design it to fit within context of use.

**Application:** Use inline help and tooltips instead of manuals. Implement progressive onboarding within the product flow. Provide contextual guidance at decision points. Design interfaces that are self-explanatory. Use empty states as teaching moments. Make help searchable and contextual, not separated.

[Reference](https://lawsofux.com/paradox-of-the-active-user/)

### UX-022: Parkinson's Law

Any task will inflate until all of the available time is spent.

**Principle:** Limit the time it takes to complete a task to what users expect. Reducing actual duration below expected duration improves overall experience. Leverage features like autofill to save time when providing critical information within forms.

**Application:** Set reasonable time constraints on tasks. Use autofill and smart defaults to speed up forms. Show estimated completion times. Reduce form fields to the minimum required. Pre-populate known information. Use single-page checkouts instead of multi-step when possible.

[Reference](https://lawsofux.com/parkinsons-law/)

### UX-023: Peak-End Rule

People judge an experience largely based on how they felt at its peak and at its end, rather than the total sum or average of every moment of the experience.

**Principle:** Pay close attention to the most intense points and final moments of the user journey. Identify moments when the product is most helpful, valuable, or entertaining and design to delight. Remember that people recall negative experiences more vividly than positive ones.

**Application:** Design delightful moments at key touchpoints (like Mailchimp's campaign send screen). Ensure the end of user flows is positive and satisfying. Minimize negative emotional peaks (like Uber reducing post-request cancellation frustration). Add surprise and delight at high-impact moments. End onboarding flows with accomplishment feelings.

[Reference](https://lawsofux.com/peak-end-rule/)

### UX-030: Zeigarnik Effect

People remember uncompleted or interrupted tasks better than completed tasks.

**Principle:** Invite content discovery by providing clear signifiers of additional content. Providing artificial progress toward a goal helps ensure users have motivation to complete that task. Provide clear indication of progress to motivate task completion.

**Application:** Use progress indicators in multi-step processes. Show incomplete profile or setup status to motivate completion. Provide visual cues for unexplored content (unread badges). Use cliffhangers in content feeds to encourage continued engagement. Show partial previews to invite exploration. Leverage incomplete states as motivation tools.

[Reference](https://lawsofux.com/zeigarnik-effect/)

## Strategic (6 laws)

### UX-001: Aesthetic-Usability Effect

Users often perceive aesthetically pleasing design as design that's more usable.

**Principle:** An aesthetically pleasing design creates a positive response in people's brains and leads them to believe the design actually works better. People are more tolerant of minor usability issues when the design is aesthetically pleasing. However, visually pleasing design can mask usability problems and prevent issues from being discovered during usability testing.

**Application:** Invest in visual polish to increase perceived usability. Use aesthetic quality to build trust and positive first impressions. But always validate with usability testing — beauty can hide real problems. Balance visual appeal with genuine functionality.

[Reference](https://lawsofux.com/aesthetic-usability-effect/)

### UX-011: Jakob's Law

Users spend most of their time on other sites. This means that users prefer your site to work the same way as all the other sites they already know.

**Principle:** Users transfer expectations from familiar products to similar ones. By leveraging existing mental models, we create superior experiences where users focus on tasks rather than learning new models. When making changes, minimize discord by allowing users to continue using familiar versions temporarily.

**Application:** Follow established design conventions and patterns. Study competitor interfaces to understand user expectations. When redesigning, allow gradual transition (like YouTube's 2017 redesign approach). Use standard form controls and navigation patterns. Don't innovate for innovation's sake — match user mental models.

[Reference](https://lawsofux.com/jakobs-law/)

### UX-019: Occam's Razor

Among competing hypotheses that predict equally well, the one with the fewest assumptions should be selected.

**Principle:** The best method for reducing complexity is to avoid it in the first place. Analyze each element and remove as many as possible without compromising function. Consider completion only when no additional items can be removed.

**Application:** Start with the simplest viable design. Remove decorative elements that don't serve a purpose. Simplify user flows by eliminating unnecessary steps. Choose the simplest solution that solves the problem. Audit existing interfaces for removable complexity. Prefer convention over configuration.

[Reference](https://lawsofux.com/occams-razor/)

### UX-021: Pareto Principle

The Pareto principle states that, for many events, roughly 80% of the effects come from 20% of the causes.

**Principle:** Inputs and outputs are often not evenly distributed. A large group may contain only a few meaningful contributors to the desired outcome. Focus the majority of effort on the areas that will bring the largest benefits to the most users.

**Application:** Identify the 20% of features that serve 80% of user needs. Prioritize fixing the most impactful usability issues first. Focus design effort on the most-used user flows. Analyze analytics to find highest-impact improvement areas. Don't over-invest in rarely-used features.

[Reference](https://lawsofux.com/pareto-principle/)

### UX-024: Postel's Law

Be liberal in what you accept, and conservative in what you send.

**Principle:** Be empathetic, flexible, and tolerant of various user actions and inputs. Anticipate virtually anything in terms of input, access, and capability while providing reliable and accessible output. The more we anticipate in design, the more resilient it will be.

**Application:** Accept multiple input formats (dates, phone numbers, addresses). Provide clear feedback on input requirements. Handle edge cases gracefully. Support various devices, browsers, and capabilities. Validate on the server side, guide on the client side. Design for accessibility and varied user needs.

[Reference](https://lawsofux.com/postels-law/)

### UX-027: Tesler's Law

For any system there is a certain amount of complexity which cannot be reduced. Also known as The Law of Conservation of Complexity.

**Principle:** All processes have core complexity that cannot be designed away and must be assumed by either the system or the user. Ensure as much burden as possible is lifted from users by handling inherent complexity during design and development. Don't build for idealized rational users.

**Application:** Identify irreducible complexity in your system. Move complexity from the user to the system where possible. Use smart defaults to reduce required decisions. Automate complex calculations and validations. Provide contextual help for unavoidably complex tasks. Accept that some complexity must remain — manage it well.

[Reference](https://lawsofux.com/teslers-law/)

