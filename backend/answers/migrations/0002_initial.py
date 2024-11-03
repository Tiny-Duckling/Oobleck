# Generated by Django 5.1.2 on 2024-11-02 06:03

import django.db.models.deletion
from django.conf import settings
from django.db import migrations, models


class Migration(migrations.Migration):

    initial = True

    dependencies = [
        ('answers', '0001_initial'),
        ('exams', '0001_initial'),
        migrations.swappable_dependency(settings.AUTH_USER_MODEL),
    ]

    operations = [
        migrations.AddField(
            model_name='answer',
            name='choice',
            field=models.ForeignKey(on_delete=django.db.models.deletion.PROTECT, to='exams.choice'),
        ),
        migrations.AddField(
            model_name='answer',
            name='question',
            field=models.ForeignKey(on_delete=django.db.models.deletion.PROTECT, to='exams.question'),
        ),
        migrations.AddField(
            model_name='answersheet',
            name='exam',
            field=models.ForeignKey(on_delete=django.db.models.deletion.PROTECT, to='exams.exam'),
        ),
        migrations.AddField(
            model_name='answersheet',
            name='examinee',
            field=models.ForeignKey(on_delete=django.db.models.deletion.PROTECT, to=settings.AUTH_USER_MODEL),
        ),
        migrations.AddField(
            model_name='answer',
            name='answer_sheet',
            field=models.ForeignKey(on_delete=django.db.models.deletion.PROTECT, to='answers.answersheet'),
        ),
    ]