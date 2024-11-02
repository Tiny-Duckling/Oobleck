from django.conf import settings
from django.db import models
from django.utils.translation import gettext_lazy


class Exam(models.Model):
    level = models.ForeignKey('questions.Level', on_delete=models.PROTECT)
    subject = models.ForeignKey('questions.Subject', on_delete=models.PROTECT)

    created_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.PROTECT)
    created_at = models.DateTimeField(auto_now_add=True)


# TODO: Update QuestionGroup's updated_by and updated_date whenever its child Question / Choice gets updated.
class QuestionGroup(models.Model):
    exam = models.ForeignKey(Exam, on_delete=models.PROTECT)
    passage_text = models.TextField()
    passage_image = models.ImageField(null=True, blank=True)

    def __str__(self):
        return self.passage_text


class Question(models.Model):
    question_group = models.ForeignKey(QuestionGroup, on_delete=models.PROTECT)
    question_text = models.TextField()
    question_image = models.ImageField(null=True, blank=True)
    solution_description = models.TextField()
    marks = models.IntegerField()
    time_limit = models.DurationField()

    solution_text = models.TextField()
    solution_value = models.DecimalField(max_digits=30, decimal_places=8)

    class Type(models.TextChoices):
        MULTIPLE_CHOICE = 'MC', gettext_lazy('Multiple Choice')
        TEXT_ANSWER = 'TXT', gettext_lazy('Text Answer')
        NUMERICAL = 'NUM', gettext_lazy('Numerical')

    type = models.CharField(max_length=3, choices=Type.choices, default=Type.MULTIPLE_CHOICE)

    def __str__(self):
        return self.question_text


class Choice(models.Model):
    multi_choice_question = models.ForeignKey(Question, on_delete=models.PROTECT, related_name='choices')
    choice_text = models.CharField(max_length=200)
    choice_image = models.ImageField(null=True, blank=True)
    is_correct = models.BooleanField(default=False)

    def __str__(self):
        return self.choice_text
