Next Tasks:
1. Fix questions
2. Learn all about permissions, difference between auth_group_permissions and profiles_user_user_permissions
3. Add superuser(done), admin, subject experts.

# Profiles
- Registration / Login / Logout / Change Password etc.
- Admin can add / edit / delete users, groups, subjects
- Subject experts can add / edit / delete topic groups, topics, questions
- Users can create groups, exams
- Users can propose subjects, topic groups, topics, questions, question bugs. If Admins / subject experts approve their propposal, they will get credits. Admins will approve subjects, subject experts will approve others.
- Users can apply to become subject experts

# Credit Management
Users can
- Get Credits upon registration (done)
- Buy Credits using a proper payment method
- Get credits upon daily / weekly / monthly / yearly login
- Gift credits to other users
- Spend credits on exams
- Get credits for adding questions approved by our subject experts
- Get credits for fixing questions approved by our subject experts
- Credit history will store all credit related operations

# Questions
Questions are mostly immutable. [Note to self: think of the ways you can make it completely immutable]
- For an update, previous question will be copied to new question with the updated fields and a leader field in old question will refer to the new question, following DSU algorithm.
- For a delete, soft delete the question.

Users with subject expertise (e.g. Physics teacher, math teacher etc.) can
- Add / Update / Delete Questions on that subject

# Exams
- Users can create exams for themselves by selecting one/more subjects, topic groups, topics, difficulty
- Users can create exams for other users / groups and pay credits upfront. Exam creator may not need to pay the full amount upfront. Credits will be stored for that particular exam. 

# Payment
- Bkash
- Rocket
- Upay
- Debit / Credit Card
- Payoneer
- Bank Transfer
- Paypal
- Stripe