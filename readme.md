# Miscellaneous
- Add time zones
- Add languages
- Add tests
- Add documentation
- Add logging
- UUID as primary key
- serializers.py for businness logic?

# Profiles
- Registration / Login / Logout / Change Password etc using **dj-rest-auth**.
- Using **rest-framework-roles** to manage permissions
- Admin can add / edit / delete users, groups, subjects
- Subject experts can add / edit / delete categories, topics, questions
- Users can create groups, exams
- Users can propose subjects, categories, topics, questions, question bugs. If Admins / subject experts approve their propposal, they will get credits.
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
- enable versioning in questions. maybe use **django-revisions**. Possible frameworks: django-simpe-history, django-reversion
- There will be 6000 unique questions for each subject
- For an update, previous question will be copied to new question with the updated fields and a leader field in old question will refer to the new question, following DSU algorithm.
- For a delete, soft delete the question.
- There will be translations of questions in several languages. maybe use **django-parler-rest**. Possible other frameworks: django-hvad, django-modeltrans, django-modeltranslation.

Users with subject expertise can
- search questions or filter using subject / categories / topics.
- approve / reject questions that is **NOT** made by himself/herself on that subject

Admins can
- approve / reject subject proposals

# Exams
- There are two types of exams: private and public.
- For private exams, exam creator needs to assign users / user groups for that exam. Credits will be deducted while assigning.
- For public exams, a public link will be created where users can take the exam by spending their own credits.
- Or, maybe an option to choose who will pay. Exam creator, or exam taker?

# Payment
- Bkash
- Rocket
- Upay
- Debit / Credit Card
- Payoneer
- Bank Transfer
- Paypal
- Stripe